use std::path::Path;

use crate::cli::report::{
    build_explain_output, format_explain_output_json, format_explain_output_report,
};
use crate::core::errors::CoreError;
use crate::core::evaluate::build_desired_snapshot_from_state;
use crate::core::reconcile::{
    normalize_verification_results_for_desired, reconcile_deterministic_plan_with_runtime,
    reconcile_plan, ReconcileDependencies,
};
use crate::core::verify::verify_state;
use crate::io::observed::build_observed_snapshot;
use crate::io::repo::HOST_OVERRIDE_ENV;
use crate::io::state::{
    latest_retained_snapshot_for_scope, read_deterministic_state, read_persisted_state,
    resolve_state_file, DETERMINISTIC_STATE_FILE_NAME,
};

pub struct ExplainCommandOutput {
    pub human: String,
    pub machine: String,
}

pub fn resolve_explain_target(
    repo: Option<&str>,
    revision: Option<&str>,
) -> Result<(String, String), CoreError> {
    if let (Some(repo), Some(revision)) = (repo, revision) {
        return Ok((repo.to_string(), revision.to_string()));
    }

    let state_path = resolve_state_file(None);
    let state = read_persisted_state(&state_path).map_err(|err| {
        CoreError::new(
            crate::core::types::FailureClass::Plan,
            format!(
                "failed to read persisted state {}: {}",
                state_path.display(),
                err
            ),
        )
    })?;
    let state = state.ok_or_else(|| {
        CoreError::new(
            crate::core::types::FailureClass::Plan,
            format!(
                "cannot infer explain target without persisted state {}; provide --repo and --rev",
                state_path.display()
            ),
        )
    })?;

    let resolved_repo = repo
        .map(ToString::to_string)
        .unwrap_or_else(|| state.desired_state.repository.clone());
    let resolved_revision = revision
        .map(ToString::to_string)
        .unwrap_or_else(|| state.desired_state.requested_ref.clone());

    if resolved_repo.trim().is_empty() || resolved_revision.trim().is_empty() {
        return Err(CoreError::new(
            crate::core::types::FailureClass::Plan,
            "persisted state does not contain a usable repository/ref; provide --repo and --rev",
        ));
    }

    Ok((resolved_repo, resolved_revision))
}

pub fn explain(
    deps: &ReconcileDependencies<'_>,
    object_selector: &str,
) -> Result<ExplainCommandOutput, CoreError> {
    let result = reconcile_plan(deps)?;
    let observed = (deps.read_observed)(&result.desired)?;
    let scope_id = scope_id_for_observed(&observed);
    let desired_snapshot = build_desired_snapshot_from_state(&result.desired, &scope_id);
    let observed_snapshot = build_observed_snapshot(&observed, Some(&result.desired), &scope_id);
    let last_applied_revision = last_applied_revision_from_state()?;
    let last_applied_snapshot = last_applied_snapshot_for_scope(&scope_id)?;
    let verification_results = normalize_verification_results_for_desired(
        &result.desired,
        verify_state(&result.desired, &observed),
    );
    let mut deterministic = reconcile_deterministic_plan_with_runtime(
        &desired_snapshot,
        last_applied_snapshot.as_ref().map(|snapshot| &snapshot.snapshot),
        &observed_snapshot,
        &verification_results,
    )?
    .plan;
    if deterministic.baseline_revision_id.is_none() {
        deterministic.baseline_revision_id = last_applied_revision;
    }
    deterministic.requested_repository = result.desired.requested_repository.clone();
    deterministic.requested_ref = result.desired.requested_ref.clone();
    deterministic.last_applied_requested_repository = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_repository.clone());
    deterministic.last_applied_requested_ref = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_ref.clone());
    let explain = build_explain_output(&deterministic, &verification_results, None, object_selector)
        .ok_or_else(|| {
            CoreError::new(
                crate::core::types::FailureClass::Plan,
                format!("managed object not found: {object_selector}"),
            )
        })?;
    Ok(ExplainCommandOutput {
        human: format_explain_output_report(&explain),
        machine: format_explain_output_json(&explain),
    })
}

fn scope_id_for_observed(observed: &crate::core::types::ObservedState) -> String {
    observed
        .host_info
        .as_ref()
        .map(|host| format!("host:{}", host.hostname))
        .or_else(|| {
            std::env::var(HOST_OVERRIDE_ENV)
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

fn last_applied_revision_from_state() -> Result<Option<String>, CoreError> {
    let state_path = resolve_state_file(None);
    read_persisted_state(&state_path)
        .map_err(|err| {
            CoreError::new(
                crate::core::types::FailureClass::Plan,
                format!(
                    "failed to read persisted state {}: {}",
                    state_path.display(),
                    err
                ),
            )
        })
        .map(|state| state.and_then(|state| state.reconciliation.last_applied_revision))
}

fn last_applied_snapshot_for_scope(
    scope_id: &str,
) -> Result<Option<crate::core::types::RetainedAppliedSnapshot>, CoreError> {
    let state_path = resolve_state_file(None);
    let deterministic_state_path = deterministic_state_path(&state_path);
    let state = read_deterministic_state(&deterministic_state_path).map_err(|err| {
        CoreError::new(
            crate::core::types::FailureClass::Plan,
            format!(
                "failed to read deterministic state {}: {}",
                deterministic_state_path.display(),
                err
            ),
        )
    })?;
    Ok(state.and_then(|state| latest_retained_snapshot_for_scope(&state, scope_id).cloned()))
}

fn deterministic_state_path(status_path: &Path) -> std::path::PathBuf {
    status_path
        .parent()
        .unwrap_or_else(|| Path::new("/var/lib/core-ops"))
        .join(DETERMINISTIC_STATE_FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::resolve_explain_target;
    use crate::io::state::{persist_success_state, STATE_FILE_ENV};

    fn temp_state_file(prefix: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        path.push(format!("{prefix}_{stamp}.json"));
        path
    }

    #[test]
    fn resolve_explain_target_prefers_explicit_values() {
        let resolved = resolve_explain_target(Some("file:///repo"), Some("demo"))
            .expect("resolve explicit target");
        assert_eq!(resolved, ("file:///repo".to_string(), "demo".to_string()));
    }

    #[test]
    fn resolve_explain_target_uses_persisted_state_for_missing_values() {
        let state_path = temp_state_file("core_ops_explain_target");
        let previous = std::env::var_os(STATE_FILE_ENV);
        std::env::set_var(STATE_FILE_ENV, &state_path);
        persist_success_state(
            &state_path,
            "file:///var/lib/core-ops/repo",
            "demo-uat-v1",
            "deadbeef",
        )
        .expect("persist state");

        let resolved = resolve_explain_target(None, None).expect("resolve from persisted state");
        assert_eq!(
            resolved,
            (
                "file:///var/lib/core-ops/repo".to_string(),
                "demo-uat-v1".to_string()
            )
        );

        let partial = resolve_explain_target(Some("file:///override"), None)
            .expect("resolve mixed target");
        assert_eq!(
            partial,
            ("file:///override".to_string(), "demo-uat-v1".to_string())
        );

        let _ = std::fs::remove_file(&state_path);
        if let Some(previous) = previous {
            std::env::set_var(STATE_FILE_ENV, previous);
        } else {
            std::env::remove_var(STATE_FILE_ENV);
        }
    }
}
