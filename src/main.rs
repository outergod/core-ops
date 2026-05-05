use clap::Parser;
use core_ops::build_info::{BUILD_REVISION, BUILD_TIME, BUILD_TREE_STATE};
use core_ops::cli::agent as agent_cmd;
use core_ops::cli::args::{Cli, Commands, SkillOp};
use core_ops::cli::common as cli_common;
use core_ops::cli::init as init_cmd;
use core_ops::cli::{apply as apply_cmd, explain as explain_cmd, plan as plan_cmd};
use core_ops::core::errors::{CoreError, StateError};
use core_ops::core::reconcile::ReconcileDependencies;
use core_ops::core::types::{FailureClass, RunStatus};
use core_ops::io::state::{
    read_persisted_state, resolve_state_file, CONTROLLER_BUILD_TIME_ENV, CONTROLLER_REVISION_ENV,
    CONTROLLER_TREE_STATE_ENV, CONTROLLER_VERSION_ENV,
};
use core_ops::io::source_ref::{detect_provenance, SourceRefError};
use core_ops::io::systemd::SYSTEMD_UNIT_DIR_ENV;
use core_ops::io::{audit as audit_io, observed, repo};
use log::LevelFilter;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

fn main() {
    set_controller_provenance_defaults();
    init_logging();
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        let report = cli_common::report_error(err);
        eprintln!("{:?}", report);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), CoreError> {
    match cli.command {
        Commands::Init(args) => {
            init_cmd::run_init(&args)?;
            println!("initialized");
            Ok(())
        }
        Commands::Plan(args) => {
            let quadlet_dir = args.quadlet_dir;
            let audit_dir = args.audit_dir;
            let json = args.json;
            let verbose = args.verbose;
            set_systemd_unit_dir(&args.systemd_unit_dir);
            set_host_override(&args.host);

            // Stateless mode (--source-repo): bypass init'd state lookup.
            // Per FR-012, writes nothing to /var/lib/core-ops/. Honors
            // --audit-dir when explicitly set (clarification Q4).
            if let Some(source_repo) = args.source_repo {
                let source = detect_provenance(&source_repo).map_err(map_source_ref_error)?;
                let repo_path = source.repo_path.clone();
                let requested_repository = source.requested_repository.clone();
                let requested_ref = source.requested_ref.clone();
                let deps = ReconcileDependencies {
                    load_desired: &|| {
                        repo::load_desired_state_from_path(
                            &repo_path,
                            &requested_repository,
                            &requested_ref,
                        )
                        .map_err(map_plan_error)
                    },
                    read_observed: &|desired| {
                        observed::read_observed_state(&quadlet_dir, Some(desired), None)
                            .map_err(map_plan_error)
                    },
                    apply_plan: &|_, _| Ok(()),
                };
                let output = plan_cmd::plan(&deps, verbose)?;
                audit_io::emit_journal_event(&output.audit_event).map_err(map_plan_error)?;
                if let Some(dir) = audit_dir {
                    let audit_path = audit_io::write_audit_record(&dir, &output.audit_record)
                        .map_err(map_plan_error)?;
                    if !json {
                        println!("audit {}", audit_path);
                    }
                }
                if json {
                    println!("{}", output.machine);
                } else {
                    println!("{}", output.summary);
                }
                return Ok(());
            }

            let (repo_source, rev) = resolve_repo_from_state(None)?;

            let deps = ReconcileDependencies {
                load_desired: &|| {
                    repo::load_desired_state(&repo_source, &rev).map_err(map_plan_error)
                },
                read_observed: &|desired| {
                    observed::read_observed_state(&quadlet_dir, Some(desired), None)
                        .map_err(map_plan_error)
                },
                apply_plan: &|_, _| Ok(()),
            };

            let output = plan_cmd::plan(&deps, verbose)?;
            audit_io::emit_journal_event(&output.audit_event).map_err(map_plan_error)?;
            if let Some(dir) = audit_dir {
                let audit_path = audit_io::write_audit_record(&dir, &output.audit_record)
                    .map_err(map_plan_error)?;
                if !json {
                    println!("audit {}", audit_path);
                }
            }

            if json {
                println!("{}", output.machine);
            } else {
                println!("{}", output.summary);
            }
            Ok(())
        }
        Commands::Apply(args) => {
            let rollback_to = args.rollback_to;
            let rollback_plan_only = args.rollback_plan_only;
            let quadlet_dir = args.quadlet_dir;
            let audit_dir = args.audit_dir;
            let json = args.json;
            let verbose = args.verbose;
            let no_reload = args.no_reload;
            set_systemd_unit_dir(&args.systemd_unit_dir);
            set_host_override(&args.host);

            // Stateless mode (--source-repo): bypass init'd state, never
            // mutate /var/lib/core-ops/status.json (FR-013, SC-009).
            // Audit records are written; the persisted controller state
            // is left byte-identical pre/post.
            if let Some(source_repo) = args.source_repo {
                if rollback_to.is_some() {
                    return Err(CoreError::new(
                        FailureClass::Apply,
                        "--rollback-to is incompatible with stateless --source-repo \
                             (rollback requires the persisted retention chain set by 'core-ops init')"
                            .to_string(),
                    ));
                }
                let source = detect_provenance(&source_repo).map_err(map_source_ref_error)?;
                let output = apply_cmd::apply_with_report_stateless(
                    &source,
                    &quadlet_dir,
                    !no_reload,
                )?;
                let run = output.result.run.clone();
                let synthetic = synthetic_provenance_for_stateless(
                    output
                        .result
                        .desired
                        .requested_repository
                        .as_deref()
                        .unwrap_or(""),
                    output
                        .result
                        .desired
                        .requested_ref
                        .as_deref()
                        .unwrap_or(""),
                );
                let event = core_ops::core::audit::build_audit_event(
                    &run,
                    Some(&output.plan),
                    &output.result.verification_results,
                    Some(&synthetic),
                );
                audit_io::emit_journal_event(&event).map_err(map_apply_error)?;
                if let Some(dir) = audit_dir {
                    let mut record = core_ops::core::audit::build_audit_record(
                        &run.run_id,
                        Vec::new(),
                        &output.plan,
                        output.result.verification_results.clone(),
                    );
                    record
                        .operator_messages
                        .push(core_ops::core::audit::summarize_evaluation(
                            &output.result.desired,
                        ));
                    let _ = audit_io::write_audit_record(&dir, &record).map_err(map_apply_error)?;
                }
                if json {
                    println!("{}", output.machine_report);
                } else if verbose {
                    println!("{}", output.verbose_report);
                } else {
                    println!("{}", output.human_report);
                }
                if run.status == RunStatus::Failure {
                    std::process::exit(1);
                }
                return Ok(());
            }

            let state_file = if args.force_no_state {
                None
            } else {
                Some(resolve_state_file(args.state_file))
            };

            let mut streamed_human_output = false;
            let output = if let Some(target_revision_id) = rollback_to.as_deref() {
                // Rollback is permitted from Detached state — only require repo (not ref) from state.
                let (repo_source, _) = resolve_repo_from_state(state_file.clone())?;
                apply_cmd::execute_rollback_with_report(
                    &repo_source,
                    target_revision_id,
                    &quadlet_dir,
                    !no_reload,
                    state_file.clone(),
                    rollback_plan_only,
                )?
            } else if json {
                let (repo_source, rev) = resolve_from_state(state_file.clone())?;
                apply_cmd::apply_with_report(
                    &repo_source,
                    &rev,
                    &quadlet_dir,
                    !no_reload,
                    state_file.clone(),
                )?
            } else {
                let (repo_source, rev) = resolve_from_state(state_file.clone())?;
                let stdout = io::stdout();
                let interactive = stdout.is_terminal();
                streamed_human_output = true;
                let mode = if verbose {
                    core_ops::cli::report::ApplyHumanMode::Verbose
                } else {
                    core_ops::cli::report::ApplyHumanMode::Default
                };
                if interactive {
                    let mut handle = io::stdout();
                    let mut spinner = InteractiveApplyDisplay::new();
                    let output = apply_cmd::apply_with_report_streaming_interactive(
                        &repo_source,
                        &rev,
                        &quadlet_dir,
                        !no_reload,
                        state_file.clone(),
                        mode,
                        |event| {
                            let _ = spinner.render(&mut handle, event);
                        },
                    )?;
                    let _ = spinner.finish(&mut handle);
                    output
                } else {
                    let mut handle = stdout.lock();
                    apply_cmd::apply_with_report_streaming(
                        &repo_source,
                        &rev,
                        &quadlet_dir,
                        !no_reload,
                        state_file.clone(),
                        mode,
                        |chunk| {
                            let _ = handle.write_all(chunk.as_bytes());
                            let _ = handle.flush();
                        },
                    )?
                }
            };
            let run = output.result.run.clone();
            let event = core_ops::core::audit::build_audit_event(
                &run,
                Some(&output.plan),
                &output.result.verification_results,
                state_file
                    .as_ref()
                    .and_then(|path| read_persisted_state(path).ok().flatten())
                    .as_ref(),
            );
            audit_io::emit_journal_event(&event).map_err(map_apply_error)?;
            if let Some(dir) = audit_dir {
                let mut record = core_ops::core::audit::build_audit_record(
                    &run.run_id,
                    Vec::new(),
                    &output.plan,
                    output.result.verification_results.clone(),
                );
                record
                    .operator_messages
                    .push(core_ops::core::audit::summarize_evaluation(
                        &output.result.desired,
                    ));
                let _ = audit_io::write_audit_record(&dir, &record).map_err(map_apply_error)?;
            }

            if json {
                println!("{}", output.machine_report);
            } else if !streamed_human_output {
                if verbose {
                    println!("{}", output.verbose_report);
                } else {
                    println!("{}", output.human_report);
                }
            }
            if run.status == RunStatus::Failure {
                std::process::exit(1);
            }
            Ok(())
        }
        Commands::Agent(args) => {
            let quadlet_dir = args
                .quadlet_dir
                .or_else(|| std::env::var_os("CORE_OPS_QUADLET_DIR").map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from("/etc/containers/systemd"));
            if let Some(systemd_unit_dir) = args
                .systemd_unit_dir
                .or_else(|| std::env::var_os(SYSTEMD_UNIT_DIR_ENV).map(PathBuf::from))
            {
                std::env::set_var(SYSTEMD_UNIT_DIR_ENV, systemd_unit_dir);
            }
            if let Some(host_override) = args.host.or_else(|| std::env::var("CORE_OPS_HOST").ok()) {
                std::env::set_var("CORE_OPS_HOST", host_override);
            }
            let audit_dir = args
                .audit_dir
                .or_else(|| std::env::var_os("CORE_OPS_AUDIT_DIR").map(PathBuf::from));
            let state_file = Some(resolve_state_file(args.state_file));
            let lock_path = args
                .lock_path
                .or_else(|| std::env::var_os("CORE_OPS_LOCK_PATH").map(PathBuf::from));

            let config = agent_cmd::AgentConfig {
                quadlet_dir,
                audit_dir,
                state_file,
                reload_systemd: !args.no_reload,
                lock_path,
            };

            match agent_cmd::run_agent(&config)? {
                agent_cmd::AgentExitReason::Uninitialized => {
                    log::info!("core-ops agent: not initialized, exiting cleanly");
                    Ok(())
                }
                agent_cmd::AgentExitReason::Detached { revision } => {
                    println!(
                        "controller is detached at revision {}; apply and agent reconciliation are paused until re-attached via 'core-ops init <repository> <ref> --force'",
                        revision
                    );
                    Ok(())
                }
                agent_cmd::AgentExitReason::Completed(output) => {
                    println!("{}", output.report);
                    if output.run.status == RunStatus::Failure {
                        std::process::exit(1);
                    }
                    Ok(())
                }
            }
        }
        Commands::Status(args) => {
            println!("{}", core_ops::cli::status::render_status(args.state_file));
            Ok(())
        }
        Commands::Explain(args) => {
            set_systemd_unit_dir(&args.systemd_unit_dir);
            set_host_override(&args.host);

            // Stateless mode (--source-repo): pure-read; writes nothing
            // anywhere. Bypasses init'd state lookup entirely.
            if let Some(source_repo) = args.source_repo {
                let source = detect_provenance(&source_repo).map_err(map_source_ref_error)?;
                let repo_path = source.repo_path.clone();
                let requested_repository = source.requested_repository.clone();
                let requested_ref = source.requested_ref.clone();
                let deps = ReconcileDependencies {
                    load_desired: &|| {
                        repo::load_desired_state_from_path(
                            &repo_path,
                            &requested_repository,
                            &requested_ref,
                        )
                        .map_err(map_plan_error)
                    },
                    read_observed: &|desired| {
                        observed::read_observed_state(&args.quadlet_dir, Some(desired), None)
                            .map_err(map_plan_error)
                    },
                    apply_plan: &|_, _| Ok(()),
                };
                let output = explain_cmd::explain(&deps, &args.object)?;
                if args.json {
                    println!("{}", output.machine);
                } else {
                    println!("{}", output.human);
                }
                return Ok(());
            }

            let (repo_source, revision) =
                explain_cmd::resolve_explain_target()?;
            let deps = ReconcileDependencies {
                load_desired: &|| {
                    repo::load_desired_state(&repo_source, &revision).map_err(map_plan_error)
                },
                read_observed: &|desired| {
                    observed::read_observed_state(&args.quadlet_dir, Some(desired), None)
                        .map_err(map_plan_error)
                },
                apply_plan: &|_, _| Ok(()),
            };
            let output = explain_cmd::explain(&deps, &args.object)?;
            if args.json {
                println!("{}", output.machine);
            } else {
                println!("{}", output.human);
            }
            Ok(())
        }
        Commands::Skill(args) => match args.op {
            SkillOp::Install(install_args) => {
                match core_ops::cli::skill::run(&install_args) {
                    Ok(()) => Ok(()),
                    Err(report) => {
                        eprintln!("{report:?}");
                        std::process::exit(1);
                    }
                }
            }
        },
    }
}

struct InteractiveApplyDisplay {
    active: Option<SpinnerHandle>,
}

struct SpinnerHandle {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl SpinnerHandle {
    fn start(line: String) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            const FRAMES: [&str; 4] = ["◰", "◳", "◲", "◱"];
            let mut index = 0usize;
            while !stop_flag.load(Ordering::Relaxed) {
                let frame = FRAMES[index % FRAMES.len()];
                let _ = write!(io::stdout(), "\r\x1b[2K{} {}", line, frame);
                let _ = io::stdout().flush();
                index += 1;
                thread::sleep(Duration::from_millis(100));
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }

    fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl InteractiveApplyDisplay {
    fn new() -> Self {
        Self { active: None }
    }

    fn render(
        &mut self,
        handle: &mut impl Write,
        event: core_ops::cli::report::ApplyInteractiveEvent,
    ) -> io::Result<()> {
        match event {
            core_ops::cli::report::ApplyInteractiveEvent::Begin(text)
            | core_ops::cli::report::ApplyInteractiveEvent::Finish(text) => {
                self.stop_active();
                handle.write_all(text.as_bytes())?;
                handle.flush()
            }
            core_ops::cli::report::ApplyInteractiveEvent::Started { line, .. } => {
                self.stop_active();
                self.active = Some(SpinnerHandle::start(line));
                Ok(())
            }
            core_ops::cli::report::ApplyInteractiveEvent::Terminal { block, .. } => {
                self.stop_active();
                handle.write_all(b"\r\x1b[2K")?;
                handle.write_all(block.as_bytes())?;
                handle.flush()
            }
        }
    }

    fn finish(&mut self, handle: &mut impl Write) -> io::Result<()> {
        self.stop_active();
        handle.flush()
    }

    fn stop_active(&mut self) {
        if let Some(active) = self.active.take() {
            active.stop();
        }
    }
}

fn init_logging() {
    if systemd_journal_logger::connected_to_journal() {
        if let Ok(logger) = systemd_journal_logger::JournalLog::new() {
            let _ = logger.install();
            log::set_max_level(LevelFilter::Info);
        }
    }
}

fn map_plan_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError::new(core_ops::core::types::FailureClass::Plan, err.to_string())
}

fn map_apply_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError::new(core_ops::core::types::FailureClass::Apply, err.to_string())
}

/// Map `--source-repo` validation errors to `CoreError`. Per
/// `contracts/cli-flag.md` the path-existence/path-shape errors exit
/// with codes 64/66; CoreError carries the message and the process
/// exit happens via the standard error-printing path. We log the
/// classified exit code into the error message so operators can see
/// it in stderr alongside the diagnostic.
fn map_source_ref_error(err: SourceRefError) -> CoreError {
    let exit_code = err.exit_code();
    CoreError::new(
        FailureClass::Plan,
        format!("{err} (exit {exit_code})"),
    )
}

/// Build a synthetic `PersistedProvenanceState` for stateless apply so
/// that audit events carry the path-based `desired_repository` and
/// `desired_requested_ref` fields without consulting any persisted
/// /var/lib/core-ops/status.json. Stateless apply never reads or
/// writes that file — the audit chain is the persisted record.
fn synthetic_provenance_for_stateless(
    requested_repository: &str,
    requested_ref: &str,
) -> core_ops::core::types::PersistedProvenanceState {
    use core_ops::core::types::{
        ControllerProvenance, DesiredStateProvenance, PersistedProvenanceState,
        ReconciliationProvenance, ReconciliationStatus, TreeState,
        PERSISTED_PROVENANCE_SCHEMA_VERSION,
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
            last_observed_revision: None,
            last_observed_at: None,
            layout_version: Some("1".to_string()),
        },
        reconciliation: ReconciliationProvenance {
            generation: 0,
            status: ReconciliationStatus::NeverRun,
            running: false,
            last_attempted_revision: None,
            last_applied_revision: None,
            last_started_at: None,
            last_finished_at: None,
            attempted_observed_divergence: None,
        },
        detached: false,
    }
}

/// Read `(repository, requested_ref)` from state, allowing Detached state.
/// Used only for the rollback path where Detached is a valid entry point.
fn resolve_repo_from_state(
    state_file_override: Option<std::path::PathBuf>,
) -> Result<(String, String), CoreError> {
    let state_path = resolve_state_file(state_file_override);
    let state = match read_persisted_state(&state_path) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Err(CoreError::new(
                FailureClass::Plan,
                format!(
                    "not initialized; run 'core-ops init <repository> <ref>' first ({})",
                    state_path.display()
                ),
            ));
        }
        Err(StateError::Corrupt(path)) => {
            return Err(CoreError::new(
                FailureClass::Plan,
                format!(
                    "state file at {} is corrupt or unreadable; run 'core-ops init <repository> <ref> --force' to recover",
                    path
                ),
            ));
        }
        Err(err) => {
            return Err(CoreError::new(FailureClass::Plan, err.to_string()));
        }
    };
    // Detached state is intentionally allowed here — rollback from Detached is valid.
    Ok((
        state.desired_state.repository,
        state.desired_state.requested_ref,
    ))
}

fn resolve_from_state(
    state_file_override: Option<std::path::PathBuf>,
) -> Result<(String, String), CoreError> {
    let state_path = resolve_state_file(state_file_override);
    let state = match read_persisted_state(&state_path) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Err(CoreError::new(
                FailureClass::Plan,
                format!(
                    "not initialized; run 'core-ops init <repository> <ref>' first ({})",
                    state_path.display()
                ),
            ));
        }
        Err(StateError::Corrupt(path)) => {
            return Err(CoreError::new(
                FailureClass::Plan,
                format!(
                    "state file at {} is corrupt or unreadable; run 'core-ops init <repository> <ref> --force' to recover",
                    path
                ),
            ));
        }
        Err(err) => {
            return Err(CoreError::new(FailureClass::Plan, err.to_string()));
        }
    };
    if state.detached {
        return Err(CoreError::new(
            FailureClass::Plan,
            format!(
                "controller is in Detached state; run 'core-ops init --force <repository> <ref>' to reinitialize ({})",
                state_path.display()
            ),
        ));
    }
    Ok((
        state.desired_state.repository,
        state.desired_state.requested_ref,
    ))
}

fn set_systemd_unit_dir(value: &Option<PathBuf>) {
    if let Some(dir) = value {
        std::env::set_var(SYSTEMD_UNIT_DIR_ENV, dir);
    }
}

fn set_host_override(value: &Option<String>) {
    if let Some(host) = value {
        std::env::set_var("CORE_OPS_HOST", host);
    }
}

fn set_controller_provenance_defaults() {
    if std::env::var_os(CONTROLLER_VERSION_ENV).is_none() {
        std::env::set_var(CONTROLLER_VERSION_ENV, canonical_controller_version());
    }
    if std::env::var_os(CONTROLLER_REVISION_ENV).is_none() {
        if let Some(revision) = BUILD_REVISION {
            std::env::set_var(CONTROLLER_REVISION_ENV, revision);
        }
    }
    if std::env::var_os(CONTROLLER_BUILD_TIME_ENV).is_none() {
        if let Some(build_time) = BUILD_TIME {
            std::env::set_var(CONTROLLER_BUILD_TIME_ENV, build_time);
        }
    }
    if std::env::var_os(CONTROLLER_TREE_STATE_ENV).is_none() {
        std::env::set_var(CONTROLLER_TREE_STATE_ENV, BUILD_TREE_STATE);
    }
}

fn canonical_controller_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
