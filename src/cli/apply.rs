use std::cell::RefCell;
use std::path::Path;

use crate::cli::report::{
    build_result_output, format_apply_output_json, format_apply_output_report,
    format_result_output_json, format_result_output_report, format_rollback_report,
    format_rollback_report_json, ApplyHumanMode, ApplyInteractiveEvent, ApplyProgressRenderer,
    ApplyRunDisplayState,
};
use crate::core::errors::CoreError;
use crate::core::evaluate::build_desired_snapshot_from_state;
use crate::core::reconcile::{
    normalize_verification_results_for_desired, reconcile_apply, reconcile_apply_with_retry,
    reconcile_deterministic_plan_with_runtime, reconcile_plan, reconcile_rollback,
    ReconcileDependencies,
};
use crate::core::types::{
    DeterministicPersistedState, FailureClass, ReconcileRun, ReconciliationStatus,
    RetainedAppliedSnapshot, RollbackTargetCandidate, RunStatus,
};
use crate::core::verify::verify_state;
use crate::io::apply::{
    apply_plan_with_desired, apply_plan_with_desired_and_events, ApplyObjectEvent,
};
use crate::io::observed::{build_observed_snapshot, read_observed_state};
use crate::io::repo::load_desired_state;
use crate::io::state::{
    latest_retained_snapshot_for_scope, persist_finished_state, persist_in_progress_state,
    read_deterministic_state, read_deterministic_state as load_deterministic_state,
    read_persisted_state, record_convergence_outcome, record_rollback_outcome, resolve_state_file,
    retain_successful_snapshot, write_deterministic_state, write_persisted_state,
    DETERMINISTIC_STATE_FILE_NAME,
};

pub fn apply(
    repo_source: &str,
    revision: &str,
    quadlet_dir: &Path,
    reload_systemd: bool,
) -> Result<ReconcileRun, CoreError> {
    let repo_source = repo_source.to_string();
    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(&repo_source, revision).map_err(map_plan_error),
        read_observed: &|desired| {
            read_observed_state(quadlet_dir, Some(desired), None).map_err(map_plan_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan_with_desired(plan, desired, quadlet_dir, reload_systemd)
                .map(|_| ())
                .map_err(map_apply_error)
        },
    };

    let result = reconcile_apply(&deps)?;
    Ok(result.run)
}

use crate::core::reconcile::ApplyResult;

const DEFAULT_RETRY_BUDGET: u32 = 3;
const DEFAULT_ROLLBACK_HISTORY: usize = 10;

pub struct ApplyReportBundle {
    pub result: ApplyResult,
    pub human_report: String,
    pub verbose_report: String,
    pub machine_report: String,
    pub result_report: String,
    pub result_machine_report: String,
    pub plan: crate::core::types::ReconciliationPlan,
}

/// Source of the desired state plus state-backend behaviour for an
/// `apply` invocation. Init'd mode (`core-ops apply` after
/// `core-ops init`) reads + writes the persisted controller state
/// at `state_path`; stateless mode (`--source-repo PATH`) keeps no
/// persistent state by design (FR-013 / SC-009 of spec/017).
///
/// All three streaming variants
/// (`apply_with_report`, `apply_with_report_streaming`,
/// `apply_with_report_streaming_interactive`) accept `&ApplyTarget`
/// so the same TTY-detection ladder in `src/main.rs` can drive both
/// modes without duplicate dispatch logic.
#[derive(Clone, Debug)]
pub enum ApplyTarget {
    Initd {
        repo_source: String,
        revision: String,
        state_path: Option<std::path::PathBuf>,
    },
    Stateless {
        source: crate::io::source_ref::StatelessSource,
    },
}

impl ApplyTarget {
    pub fn initd(
        repo_source: impl Into<String>,
        revision: impl Into<String>,
        state_path: Option<std::path::PathBuf>,
    ) -> Self {
        Self::Initd {
            repo_source: repo_source.into(),
            revision: revision.into(),
            state_path,
        }
    }

    pub fn stateless(source: crate::io::source_ref::StatelessSource) -> Self {
        Self::Stateless { source }
    }

    pub fn state_path(&self) -> Option<&Path> {
        match self {
            Self::Initd { state_path, .. } => state_path.as_deref(),
            Self::Stateless { .. } => None,
        }
    }

    pub fn is_stateless(&self) -> bool {
        matches!(self, Self::Stateless { .. })
    }

    /// String to record as the apply event's `repo_source` (audit /
    /// persisted state). For init'd mode, the operator's repo URL or
    /// path; for stateless mode, the canonical source-repo path.
    pub fn repo_source_for_record(&self) -> &str {
        match self {
            Self::Initd { repo_source, .. } => repo_source,
            Self::Stateless { source } => &source.requested_repository,
        }
    }

    /// Revision string to record. For init'd mode, the operator's
    /// requested ref. For stateless mode, the path-based provenance
    /// sentinel (`<sha>` / `(stateless)` / `(stateless+dirty)`).
    pub fn revision_for_record(&self) -> &str {
        match self {
            Self::Initd { revision, .. } => revision,
            Self::Stateless { source } => &source.requested_ref,
        }
    }

    fn load_desired(
        &self,
    ) -> Result<crate::core::types::DesiredState, crate::core::errors::CoreError> {
        match self {
            Self::Initd {
                repo_source,
                revision,
                ..
            } => load_desired_state(repo_source, revision).map_err(map_plan_error),
            Self::Stateless { source } => crate::io::repo::load_desired_state_from_path(
                &source.repo_path,
                &source.requested_repository,
                &source.requested_ref,
            )
            // Per `contracts/cli-flag.md` Error semantics: a path that
            // exists as a directory but is not a spec/016-conformant
            // source-repo (missing services/, legacy artifacts, etc.)
            // exits 65 (`EX_DATAERR`).
            .map_err(|err| CoreError::with_exit_code(FailureClass::Plan, err.to_string(), 65)),
        }
    }

    /// Pick the run-display state for header rendering. Init'd mode
    /// uses the three-way `last_applied + observed` heuristic; stateless
    /// mode uses the dedicated `Stateless` variant when the host has
    /// observed objects (the controller cannot distinguish a successful
    /// prior apply from a failed one without `/var/lib/core-ops/`),
    /// falling back to `FirstRun` for an empty host.
    fn classify_run_display(
        &self,
        last_applied: Option<&str>,
        observed: &crate::core::types::NormalizedSnapshot,
    ) -> ApplyRunDisplayState {
        if self.is_stateless() {
            if observed.objects.is_empty() {
                ApplyRunDisplayState::FirstRun
            } else {
                ApplyRunDisplayState::Stateless
            }
        } else {
            classify_apply_run_display_state(last_applied, observed)
        }
    }
}

pub fn apply_with_report(
    target: &ApplyTarget,
    quadlet_dir: &Path,
    reload_systemd: bool,
) -> Result<ApplyReportBundle, CoreError> {
    let deps = ReconcileDependencies {
        load_desired: &|| target.load_desired(),
        read_observed: &|desired| {
            read_observed_state(quadlet_dir, Some(desired), None).map_err(map_plan_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan_with_desired(plan, desired, quadlet_dir, reload_systemd)
                .map(|_| ())
                .map_err(map_apply_error)
        },
    };

    let plan_result = reconcile_plan(&deps)?;
    let observed_before = (deps.read_observed)(&plan_result.desired)?;
    let scope_id = scope_id_for_observed(&observed_before);
    let desired_snapshot = build_desired_snapshot_from_state(&plan_result.desired, &scope_id);
    let observed_snapshot =
        build_observed_snapshot(&observed_before, Some(&plan_result.desired), &scope_id);
    let state_path = target.state_path();
    let last_applied_revision = last_applied_revision_from_state(state_path)?;
    let last_applied_snapshot = last_applied_snapshot_for_scope(state_path, &scope_id)?;
    let run_display_state =
        target.classify_run_display(last_applied_revision.as_deref(), &observed_snapshot);
    let verification_results_before = normalize_verification_results_for_desired(
        &plan_result.desired,
        verify_state(&plan_result.desired, &observed_before),
    );
    let baseline_snapshot = last_applied_snapshot
        .as_ref()
        .map(|snapshot| &snapshot.snapshot);
    let mut deterministic = reconcile_deterministic_plan_with_runtime(
        &desired_snapshot,
        baseline_snapshot,
        &observed_snapshot,
        &verification_results_before,
    )?
    .plan;
    if deterministic.baseline_revision_id.is_none() {
        deterministic.baseline_revision_id = last_applied_revision;
    }
    deterministic.requested_repository = plan_result.desired.requested_repository.clone();
    deterministic.requested_ref = plan_result.desired.requested_ref.clone();
    deterministic.last_applied_requested_repository = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_repository.clone());
    deterministic.last_applied_requested_ref = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_ref.clone());
    let attempt = match state_path {
        Some(path) => Some(
            persist_in_progress_state(
                path,
                target.repo_source_for_record(),
                target.revision_for_record(),
                &plan_result.desired.revision_id,
                None,
            )
            .map_err(map_apply_error)?,
        ),
        None => None,
    };
    let result = reconcile_apply_with_retry(&deps, DEFAULT_RETRY_BUDGET)?;
    if result
        .desired
        .mount_declarations
        .iter()
        .any(|mount| mount.automount)
    {
        deterministic.scope_id = scope_id.clone();
    }
    let human_report = format_apply_output_report(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
        ApplyHumanMode::Default,
        run_display_state,
    );
    let verbose_report = format_apply_output_report(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
        ApplyHumanMode::Verbose,
        run_display_state,
    );
    let machine_report = format_apply_output_json(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
    );
    let result_view = build_result_output(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
    );
    let result_report = format_result_output_report(&result_view);
    let result_machine_report = format_result_output_json(&result_view);

    if let Some(status_path) = state_path {
        let deterministic_state_path = deterministic_state_path(status_path);
        let mut deterministic_state =
            load_or_init_deterministic_state(&deterministic_state_path).map_err(map_apply_error)?;
        if let Some(convergence) = result.convergence.clone() {
            let desired_snapshot =
                build_desired_snapshot_from_state(&result.desired, &convergence.scope_id);
            record_convergence_outcome(&mut deterministic_state, convergence);
            if result.run.status == RunStatus::Success {
                retain_successful_snapshot(
                    &mut deterministic_state,
                    RetainedAppliedSnapshot {
                        revision_id: result.desired.revision_id.clone(),
                        scope_id: desired_snapshot.scope_id.clone(),
                        requested_repository: result.desired.requested_repository.clone(),
                        requested_ref: result.desired.requested_ref.clone(),
                        snapshot: desired_snapshot,
                        retained: true,
                    },
                    DEFAULT_ROLLBACK_HISTORY,
                );
            }
            write_deterministic_state(&deterministic_state_path, &deterministic_state)
                .map_err(map_apply_error)?;
        }
    }
    if let (Some(path), Some(attempt)) = (state_path, attempt.as_ref()) {
        let status = match result.run.status {
            RunStatus::Success => ReconciliationStatus::Success,
            RunStatus::Failure => ReconciliationStatus::Failed,
        };
        persist_finished_state(
            path,
            target.repo_source_for_record(),
            target.revision_for_record(),
            &result.desired.revision_id,
            None,
            attempt,
            status,
        )
        .map_err(map_apply_error)?;
    }
    Ok(ApplyReportBundle {
        result,
        human_report,
        verbose_report,
        machine_report,
        result_report,
        result_machine_report,
        plan: plan_result.plan,
    })
}

pub fn apply_with_report_streaming<F>(
    target: &ApplyTarget,
    quadlet_dir: &Path,
    reload_systemd: bool,
    mode: ApplyHumanMode,
    emit: F,
) -> Result<ApplyReportBundle, CoreError>
where
    F: FnMut(&str),
{
    let plan_deps = ReconcileDependencies {
        load_desired: &|| target.load_desired(),
        read_observed: &|desired| {
            read_observed_state(quadlet_dir, Some(desired), None).map_err(map_plan_error)
        },
        apply_plan: &|_, _| Ok(()),
    };
    let plan_result = reconcile_plan(&plan_deps)?;
    let observed_before = (plan_deps.read_observed)(&plan_result.desired)?;
    let scope_id = scope_id_for_observed(&observed_before);
    let desired_snapshot = build_desired_snapshot_from_state(&plan_result.desired, &scope_id);
    let observed_snapshot =
        build_observed_snapshot(&observed_before, Some(&plan_result.desired), &scope_id);
    let state_path = target.state_path();
    let last_applied_revision = last_applied_revision_from_state(state_path)?;
    let last_applied_snapshot = last_applied_snapshot_for_scope(state_path, &scope_id)?;
    let run_display_state =
        target.classify_run_display(last_applied_revision.as_deref(), &observed_snapshot);
    let verification_results_before = normalize_verification_results_for_desired(
        &plan_result.desired,
        verify_state(&plan_result.desired, &observed_before),
    );
    let baseline_snapshot = last_applied_snapshot
        .as_ref()
        .map(|snapshot| &snapshot.snapshot);
    let mut deterministic = reconcile_deterministic_plan_with_runtime(
        &desired_snapshot,
        baseline_snapshot,
        &observed_snapshot,
        &verification_results_before,
    )?
    .plan;
    if deterministic.baseline_revision_id.is_none() {
        deterministic.baseline_revision_id = last_applied_revision;
    }
    deterministic.requested_repository = plan_result.desired.requested_repository.clone();
    deterministic.requested_ref = plan_result.desired.requested_ref.clone();
    deterministic.last_applied_requested_repository = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_repository.clone());
    deterministic.last_applied_requested_ref = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_ref.clone());
    let renderer = RefCell::new(ApplyProgressRenderer::new(
        &deterministic,
        mode,
        run_display_state,
    ));
    let emit = RefCell::new(emit);
    emit.borrow_mut()(&renderer.borrow().begin());

    let stream_quadlet_dir = quadlet_dir.to_path_buf();
    let deps = ReconcileDependencies {
        load_desired: &|| target.load_desired(),
        read_observed: &|desired| {
            read_observed_state(&stream_quadlet_dir, Some(desired), None).map_err(map_plan_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan_with_desired_and_events(
                plan,
                desired,
                &stream_quadlet_dir,
                reload_systemd,
                |event| match event {
                    ApplyObjectEvent::Started { target } => {
                        if let Some(chunk) = renderer.borrow_mut().render_started(&target) {
                            emit.borrow_mut()(&chunk);
                        }
                    }
                    ApplyObjectEvent::Completed { target } => {
                        if let Some(chunk) = renderer.borrow_mut().render_completed(&target) {
                            emit.borrow_mut()(&chunk);
                        }
                    }
                    ApplyObjectEvent::Failed { target, error } => {
                        if let Some(chunk) = renderer.borrow_mut().render_failed(&target, &error) {
                            emit.borrow_mut()(&chunk);
                        }
                    }
                },
            )
            .map(|_| ())
            .map_err(map_apply_error)
        },
    };

    let attempt = match state_path {
        Some(path) => Some(
            persist_in_progress_state(
                path,
                target.repo_source_for_record(),
                target.revision_for_record(),
                &plan_result.desired.revision_id,
                None,
            )
            .map_err(map_apply_error)?,
        ),
        None => None,
    };
    let result = reconcile_apply_with_retry(&deps, DEFAULT_RETRY_BUDGET)?;
    if result
        .desired
        .mount_declarations
        .iter()
        .any(|mount| mount.automount)
    {
        deterministic.scope_id = scope_id.clone();
    }
    emit.borrow_mut()(
        &renderer
            .borrow_mut()
            .finish(&result.verification_results, result.convergence.as_ref()),
    );

    let human_report = format_apply_output_report(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
        ApplyHumanMode::Default,
        run_display_state,
    );
    let verbose_report = format_apply_output_report(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
        ApplyHumanMode::Verbose,
        run_display_state,
    );
    let machine_report = format_apply_output_json(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
    );
    let result_view = build_result_output(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
    );
    let result_report = format_result_output_report(&result_view);
    let result_machine_report = format_result_output_json(&result_view);

    if let Some(status_path) = state_path {
        let deterministic_state_path = deterministic_state_path(status_path);
        let mut deterministic_state =
            load_or_init_deterministic_state(&deterministic_state_path).map_err(map_apply_error)?;
        if let Some(convergence) = result.convergence.clone() {
            let desired_snapshot =
                build_desired_snapshot_from_state(&result.desired, &convergence.scope_id);
            record_convergence_outcome(&mut deterministic_state, convergence);
            if result.run.status == RunStatus::Success {
                retain_successful_snapshot(
                    &mut deterministic_state,
                    RetainedAppliedSnapshot {
                        revision_id: result.desired.revision_id.clone(),
                        scope_id: desired_snapshot.scope_id.clone(),
                        requested_repository: result.desired.requested_repository.clone(),
                        requested_ref: result.desired.requested_ref.clone(),
                        snapshot: desired_snapshot,
                        retained: true,
                    },
                    DEFAULT_ROLLBACK_HISTORY,
                );
            }
            write_deterministic_state(&deterministic_state_path, &deterministic_state)
                .map_err(map_apply_error)?;
        }
    }
    if let (Some(path), Some(attempt)) = (state_path, attempt.as_ref()) {
        let status = match result.run.status {
            RunStatus::Success => ReconciliationStatus::Success,
            RunStatus::Failure => ReconciliationStatus::Failed,
        };
        persist_finished_state(
            path,
            target.repo_source_for_record(),
            target.revision_for_record(),
            &result.desired.revision_id,
            None,
            attempt,
            status,
        )
        .map_err(map_apply_error)?;
    }

    Ok(ApplyReportBundle {
        result,
        human_report,
        verbose_report,
        machine_report,
        result_report,
        result_machine_report,
        plan: plan_result.plan,
    })
}

pub fn apply_with_report_streaming_interactive<F>(
    target: &ApplyTarget,
    quadlet_dir: &Path,
    reload_systemd: bool,
    mode: ApplyHumanMode,
    emit: F,
) -> Result<ApplyReportBundle, CoreError>
where
    F: FnMut(ApplyInteractiveEvent),
{
    let plan_deps = ReconcileDependencies {
        load_desired: &|| target.load_desired(),
        read_observed: &|desired| {
            read_observed_state(quadlet_dir, Some(desired), None).map_err(map_plan_error)
        },
        apply_plan: &|_, _| Ok(()),
    };
    let plan_result = reconcile_plan(&plan_deps)?;
    let observed_before = (plan_deps.read_observed)(&plan_result.desired)?;
    let scope_id = scope_id_for_observed(&observed_before);
    let desired_snapshot = build_desired_snapshot_from_state(&plan_result.desired, &scope_id);
    let observed_snapshot =
        build_observed_snapshot(&observed_before, Some(&plan_result.desired), &scope_id);
    let state_path = target.state_path();
    let last_applied_revision = last_applied_revision_from_state(state_path)?;
    let last_applied_snapshot = last_applied_snapshot_for_scope(state_path, &scope_id)?;
    let run_display_state =
        target.classify_run_display(last_applied_revision.as_deref(), &observed_snapshot);
    let verification_results_before = normalize_verification_results_for_desired(
        &plan_result.desired,
        verify_state(&plan_result.desired, &observed_before),
    );
    let baseline_snapshot = last_applied_snapshot
        .as_ref()
        .map(|snapshot| &snapshot.snapshot);
    let mut deterministic = reconcile_deterministic_plan_with_runtime(
        &desired_snapshot,
        baseline_snapshot,
        &observed_snapshot,
        &verification_results_before,
    )?
    .plan;
    if deterministic.baseline_revision_id.is_none() {
        deterministic.baseline_revision_id = last_applied_revision;
    }
    deterministic.requested_repository = plan_result.desired.requested_repository.clone();
    deterministic.requested_ref = plan_result.desired.requested_ref.clone();
    deterministic.last_applied_requested_repository = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_repository.clone());
    deterministic.last_applied_requested_ref = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_ref.clone());
    let renderer = RefCell::new(ApplyProgressRenderer::new(
        &deterministic,
        mode,
        run_display_state,
    ));
    let emit = RefCell::new(emit);
    emit.borrow_mut()(renderer.borrow().begin_interactive());

    let stream_quadlet_dir = quadlet_dir.to_path_buf();
    let deps = ReconcileDependencies {
        load_desired: &|| target.load_desired(),
        read_observed: &|desired| {
            read_observed_state(&stream_quadlet_dir, Some(desired), None).map_err(map_plan_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan_with_desired_and_events(
                plan,
                desired,
                &stream_quadlet_dir,
                reload_systemd,
                |event| match event {
                    ApplyObjectEvent::Started { target } => {
                        if let Some(event) =
                            renderer.borrow_mut().render_started_interactive(&target)
                        {
                            emit.borrow_mut()(event);
                        }
                    }
                    ApplyObjectEvent::Completed { target } => {
                        if let Some(event) =
                            renderer.borrow_mut().render_completed_interactive(&target)
                        {
                            emit.borrow_mut()(event);
                        }
                    }
                    ApplyObjectEvent::Failed { target, error } => {
                        if let Some(event) = renderer
                            .borrow_mut()
                            .render_failed_interactive(&target, &error)
                        {
                            emit.borrow_mut()(event);
                        }
                    }
                },
            )
            .map(|_| ())
            .map_err(map_apply_error)
        },
    };

    let attempt = match state_path {
        Some(path) => Some(
            persist_in_progress_state(
                path,
                target.repo_source_for_record(),
                target.revision_for_record(),
                &plan_result.desired.revision_id,
                None,
            )
            .map_err(map_apply_error)?,
        ),
        None => None,
    };
    let result = reconcile_apply_with_retry(&deps, DEFAULT_RETRY_BUDGET)?;
    if result
        .desired
        .mount_declarations
        .iter()
        .any(|mount| mount.automount)
    {
        deterministic.scope_id = scope_id.clone();
    }
    emit.borrow_mut()(
        renderer
            .borrow_mut()
            .finish_interactive(&result.verification_results, result.convergence.as_ref()),
    );

    let human_report = format_apply_output_report(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
        ApplyHumanMode::Default,
        run_display_state,
    );
    let verbose_report = format_apply_output_report(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
        ApplyHumanMode::Verbose,
        run_display_state,
    );
    let machine_report = format_apply_output_json(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
    );
    let result_view = build_result_output(
        &deterministic,
        &result.verification_results,
        result.convergence.as_ref(),
    );
    let result_report = format_result_output_report(&result_view);
    let result_machine_report = format_result_output_json(&result_view);

    if let Some(status_path) = state_path {
        let deterministic_state_path = deterministic_state_path(status_path);
        let mut deterministic_state =
            load_or_init_deterministic_state(&deterministic_state_path).map_err(map_apply_error)?;
        if let Some(convergence) = result.convergence.clone() {
            let desired_snapshot =
                build_desired_snapshot_from_state(&result.desired, &convergence.scope_id);
            record_convergence_outcome(&mut deterministic_state, convergence);
            if result.run.status == RunStatus::Success {
                retain_successful_snapshot(
                    &mut deterministic_state,
                    RetainedAppliedSnapshot {
                        revision_id: result.desired.revision_id.clone(),
                        scope_id: desired_snapshot.scope_id.clone(),
                        requested_repository: result.desired.requested_repository.clone(),
                        requested_ref: result.desired.requested_ref.clone(),
                        snapshot: desired_snapshot,
                        retained: true,
                    },
                    DEFAULT_ROLLBACK_HISTORY,
                );
            }
            write_deterministic_state(&deterministic_state_path, &deterministic_state)
                .map_err(map_apply_error)?;
        }
    }
    if let (Some(path), Some(attempt)) = (state_path, attempt.as_ref()) {
        let status = match result.run.status {
            RunStatus::Success => ReconciliationStatus::Success,
            RunStatus::Failure => ReconciliationStatus::Failed,
        };
        persist_finished_state(
            path,
            target.repo_source_for_record(),
            target.revision_for_record(),
            &result.desired.revision_id,
            None,
            attempt,
            status,
        )
        .map_err(map_apply_error)?;
    }

    Ok(ApplyReportBundle {
        result,
        human_report,
        verbose_report,
        machine_report,
        result_report,
        result_machine_report,
        plan: plan_result.plan,
    })
}

pub fn rollback_with_report(
    deterministic_state_path: &Path,
    target_revision_id: &str,
    actual: &crate::core::types::NormalizedSnapshot,
) -> Result<String, CoreError> {
    let state = read_deterministic_state(deterministic_state_path)
        .map_err(map_apply_error)?
        .ok_or_else(|| {
            CoreError::new(
                FailureClass::Apply,
                "deterministic rollback state is absent",
            )
        })?;
    let result = reconcile_rollback(&state, &actual.scope_id, target_revision_id, actual)?;
    Ok(format_rollback_report(&result.target, &result.plan))
}

pub fn execute_rollback_with_report(
    repo_source: &str,
    target_revision_id: &str,
    quadlet_dir: &Path,
    reload_systemd: bool,
    state_path: Option<std::path::PathBuf>,
    plan_only: bool,
) -> Result<ApplyReportBundle, CoreError> {
    let state_path = state_path.ok_or_else(|| {
        CoreError::new(
            FailureClass::Apply,
            "rollback requires a persisted state path and deterministic history",
        )
    })?;
    let deterministic_state_path = deterministic_state_path(&state_path);
    let desired = load_desired_state(repo_source, target_revision_id).map_err(map_plan_error)?;
    let observed =
        read_observed_state(quadlet_dir, Some(&desired), None).map_err(map_plan_error)?;
    let actual =
        build_observed_snapshot(&observed, Some(&desired), &scope_id_for_observed(&observed));
    let rollback_state = read_deterministic_state(&deterministic_state_path)
        .map_err(map_apply_error)?
        .ok_or_else(|| {
            CoreError::new(
                FailureClass::Apply,
                "deterministic rollback state is absent",
            )
        })?;
    let rollback_preview = reconcile_rollback(
        &rollback_state,
        &actual.scope_id,
        target_revision_id,
        &actual,
    )?;
    let preview = format_rollback_report(&rollback_preview.target, &rollback_preview.plan);
    let preview_json =
        format_rollback_report_json(&rollback_preview.target, &rollback_preview.plan);

    if plan_only {
        let run = ReconcileRun {
            run_id: format!("run:rollback-plan:{target_revision_id}"),
            mode: crate::core::types::ReconcileMode::Plan,
            status: RunStatus::Success,
            failure_class: None,
            summary: format!("rollback plan ready for {}", target_revision_id),
        };
        return Ok(ApplyReportBundle {
            result: ApplyResult {
                run,
                verification_results: Vec::new(),
                desired,
                convergence: None,
                plan: reconcile_plan(&ReconcileDependencies {
                    load_desired: &|| {
                        load_desired_state(repo_source, target_revision_id).map_err(map_plan_error)
                    },
                    read_observed: &|desired| {
                        read_observed_state(quadlet_dir, Some(desired), None)
                            .map_err(map_plan_error)
                    },
                    apply_plan: &|plan, desired| {
                        apply_plan_with_desired(plan, desired, quadlet_dir, reload_systemd)
                            .map(|_| ())
                            .map_err(map_apply_error)
                    },
                })?
                .plan,
            },
            human_report: preview.clone(),
            verbose_report: preview,
            machine_report: preview_json,
            result_report: String::new(),
            result_machine_report: String::new(),
            plan: reconcile_plan(&ReconcileDependencies {
                load_desired: &|| {
                    load_desired_state(repo_source, target_revision_id).map_err(map_plan_error)
                },
                read_observed: &|desired| {
                    read_observed_state(quadlet_dir, Some(desired), None).map_err(map_plan_error)
                },
                apply_plan: &|plan, desired| {
                    apply_plan_with_desired(plan, desired, quadlet_dir, reload_systemd)
                        .map(|_| ())
                        .map_err(map_apply_error)
                },
            })?
            .plan,
        });
    }

    let output = apply_with_report(
        &ApplyTarget::initd(repo_source, target_revision_id, Some(state_path.clone())),
        quadlet_dir,
        reload_systemd,
    )?;

    // After a successful rollback, mark controller as Detached.
    // This write is part of rollback success criteria — failure here means the controller
    // may remain attached and the next agent tick could reconcile away the recovered snapshot.
    if output.result.run.status == RunStatus::Success {
        match read_persisted_state(&state_path) {
            Ok(Some(mut provenance)) => {
                provenance.detached = true;
                write_persisted_state(&state_path, &provenance).map_err(|err| {
                    CoreError::new(
                        FailureClass::Apply,
                        format!(
                            "rollback succeeded but failed to persist detached state to {}: {}; \
                             re-run rollback to recover",
                            state_path.display(),
                            err
                        ),
                    )
                })?;
            }
            Ok(None) => {}
            Err(err) => {
                return Err(CoreError::new(
                    FailureClass::Apply,
                    format!(
                        "rollback succeeded but could not read state at {} to mark detached: {}; \
                         re-run rollback to recover",
                        state_path.display(),
                        err
                    ),
                ));
            }
        }
    }

    let mut deterministic_state =
        load_or_init_deterministic_state(&deterministic_state_path).map_err(map_apply_error)?;
    if let Some(convergence) = output.result.convergence.clone() {
        record_rollback_outcome(
            &mut deterministic_state,
            RollbackTargetCandidate {
                target_revision_id: target_revision_id.to_string(),
                scope_id: convergence.scope_id.clone(),
                eligibility: crate::core::types::RollbackEligibility::Eligible,
                reason: "retained successful snapshot is rollback-eligible".to_string(),
            },
            convergence,
        );
        write_deterministic_state(&deterministic_state_path, &deterministic_state)
            .map_err(map_apply_error)?;
    }

    Ok(ApplyReportBundle {
        human_report: format!("{preview}\n{}", output.human_report),
        verbose_report: format!("{preview}\n{}", output.verbose_report),
        ..output
    })
}

/// Build a synthetic `PersistedProvenanceState` for stateless apply
/// so that the audit event surfaces path-based provenance plus the
/// actual run outcome (success / failure) without consulting any
/// persisted `/var/lib/core-ops/status.json`.
///
/// `revision_id` is the resolved desired-state revision (a SHA, a
/// `(stateless)` / `(stateless+dirty)` sentinel, or a synthetic id
/// from `load_desired_state_from_path`); `run_status` is the apply
/// outcome. Together they populate the audit event's
/// `reconciliation_status`, `attempted_revision`, and
/// `applied_revision` fields with values that reflect what actually
/// happened — fixing the prior bug where every stateless apply
/// emitted `reconciliation_status = "never_run"`.
pub fn synthetic_stateless_provenance(
    requested_repository: &str,
    requested_ref: &str,
    revision_id: &str,
    run_status: RunStatus,
) -> crate::core::types::PersistedProvenanceState {
    use crate::core::types::{
        ControllerProvenance, DesiredStateProvenance, PersistedProvenanceState,
        ReconciliationProvenance, ReconciliationStatus, TreeState,
        PERSISTED_PROVENANCE_SCHEMA_VERSION,
    };
    let reconciliation_status = match run_status {
        RunStatus::Success => ReconciliationStatus::Success,
        RunStatus::Failure => ReconciliationStatus::Failed,
    };
    let last_applied_revision = match run_status {
        RunStatus::Success => Some(revision_id.to_string()),
        RunStatus::Failure => None,
    };
    PersistedProvenanceState {
        schema_version: PERSISTED_PROVENANCE_SCHEMA_VERSION,
        controller: ControllerProvenance {
            version: None,
            revision: None,
            build_time: None,
            tree_state: TreeState::Unknown,
        },
        desired_state: DesiredStateProvenance {
            repository: requested_repository.to_string(),
            requested_ref: requested_ref.to_string(),
            last_observed_revision: Some(revision_id.to_string()),
            last_observed_at: None,
            layout_version: Some("1".to_string()),
        },
        reconciliation: ReconciliationProvenance {
            // Stateless mode has no prior generation context — this
            // is a single ad-hoc run, semantically generation 1.
            generation: 1,
            status: reconciliation_status,
            running: false,
            last_attempted_revision: Some(revision_id.to_string()),
            last_applied_revision,
            last_started_at: None,
            last_finished_at: None,
            attempted_observed_divergence: None,
        },
        detached: false,
    }
}

fn classify_apply_run_display_state(
    last_applied_revision: Option<&str>,
    observed_snapshot: &crate::core::types::NormalizedSnapshot,
) -> ApplyRunDisplayState {
    if last_applied_revision.is_some() {
        ApplyRunDisplayState::Managed
    } else if observed_snapshot.objects.is_empty() {
        ApplyRunDisplayState::FirstRun
    } else {
        ApplyRunDisplayState::Recovery
    }
}

fn deterministic_state_path(status_path: &Path) -> std::path::PathBuf {
    status_path
        .parent()
        .unwrap_or_else(|| Path::new("/var/lib/core-ops"))
        .join(DETERMINISTIC_STATE_FILE_NAME)
}

fn last_applied_revision_from_state(
    status_path: Option<&Path>,
) -> Result<Option<String>, CoreError> {
    let Some(path) = status_path else {
        return Ok(None);
    };
    crate::io::state::read_persisted_state(path)
        .map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to read persisted state {}: {}", path.display(), err),
            )
        })
        .map(|state| state.and_then(|state| state.reconciliation.last_applied_revision))
}

fn last_applied_snapshot_for_scope(
    status_path: Option<&Path>,
    scope_id: &str,
) -> Result<Option<RetainedAppliedSnapshot>, CoreError> {
    let path = status_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| resolve_state_file(None));
    let deterministic_state_path = deterministic_state_path(&path);
    let state = read_deterministic_state(&deterministic_state_path).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to read deterministic state {}: {}",
                deterministic_state_path.display(),
                err
            ),
        )
    })?;
    Ok(state.and_then(|state| latest_retained_snapshot_for_scope(&state, scope_id).cloned()))
}

fn load_or_init_deterministic_state(
    path: &Path,
) -> Result<DeterministicPersistedState, crate::core::errors::StateError> {
    load_deterministic_state(path)?.map_or_else(
        || {
            Ok(DeterministicPersistedState {
                schema_version: 1,
                current_scope: default_host_scope_id()
                    .unwrap_or_else(|| "scope:default".to_string()),
                retained_snapshots: Vec::new(),
                latest_convergence: None,
                latest_rollback_target: None,
            })
        },
        Ok,
    )
}

fn scope_id_for_observed(observed: &crate::core::types::ObservedState) -> String {
    observed
        .host_info
        .as_ref()
        .map(|host| format!("host:{}", host.hostname))
        .or_else(|| {
            std::env::var(crate::io::repo::HOST_OVERRIDE_ENV)
                .ok()
                .filter(|host| !host.is_empty())
                .map(|host| format!("host:{host}"))
        })
        .or_else(default_host_scope_id)
        .unwrap_or_else(|| "scope:default".to_string())
}

fn default_host_scope_id() -> Option<String> {
    let mut buf = [0u8; 256];
    let result = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if result != 0 {
        return None;
    }
    let len = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let hostname = String::from_utf8_lossy(&buf[..len]).trim().to_string();
    (!hostname.is_empty()).then(|| format!("host:{hostname}"))
}

fn map_plan_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError::new(FailureClass::Plan, err.to_string())
}

fn map_apply_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError::new(FailureClass::Apply, err.to_string())
}
