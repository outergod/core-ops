use clap::Parser;
use core_ops::build_info::{BUILD_REVISION, BUILD_TIME, BUILD_TREE_STATE};
use core_ops::cli::agent as agent_cmd;
use core_ops::cli::args::{Cli, Commands};
use core_ops::cli::common as cli_common;
use core_ops::cli::{apply as apply_cmd, explain as explain_cmd, plan as plan_cmd};
use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::ReconcileDependencies;
use core_ops::core::types::RunStatus;
use core_ops::io::state::{
    read_persisted_state, resolve_state_file, CONTROLLER_BUILD_TIME_ENV, CONTROLLER_REVISION_ENV,
    CONTROLLER_TREE_STATE_ENV, CONTROLLER_VERSION_ENV,
};
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
        Commands::Plan(args) => {
            let repo_source = args.repo;
            let rev = args.rev;
            let quadlet_dir = args.quadlet_dir;
            let audit_dir = args.audit_dir;
            let json = args.json;
            let verbose = args.verbose;
            set_systemd_unit_dir(&args.systemd_unit_dir);
            set_host_override(&args.host);

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
                println!("audit {}", audit_path);
            }

            if json {
                println!("{}", output.machine);
            } else {
                println!("{}", output.summary);
            }
            Ok(())
        }
        Commands::Apply(args) => {
            let repo_source = args.repo;
            let rev = args.rev;
            let rollback_to = args.rollback_to;
            let rollback_plan_only = args.rollback_plan_only;
            let quadlet_dir = args.quadlet_dir;
            let audit_dir = args.audit_dir;
            let json = args.json;
            let verbose = args.verbose;
            let state_file = if args.force_no_state {
                None
            } else {
                Some(resolve_state_file(args.state_file))
            };
            let no_reload = args.no_reload;
            set_systemd_unit_dir(&args.systemd_unit_dir);
            set_host_override(&args.host);

            let mut streamed_human_output = false;
            let output = if let Some(target_revision_id) = rollback_to.as_deref() {
                apply_cmd::execute_rollback_with_report(
                    &repo_source,
                    target_revision_id,
                    &quadlet_dir,
                    !no_reload,
                    state_file.clone(),
                    rollback_plan_only,
                )?
            } else if json {
                apply_cmd::apply_with_report(
                    &repo_source,
                    &rev,
                    &quadlet_dir,
                    !no_reload,
                    state_file.clone(),
                )?
            } else {
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
            let repo = resolve_env(args.repo, "CORE_OPS_REPO")?;
            let rev = resolve_env(args.rev, "CORE_OPS_REV")?;
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
                repo,
                rev,
                quadlet_dir,
                audit_dir,
                state_file,
                reload_systemd: !args.no_reload,
                lock_path,
            };

            let output = agent_cmd::run_agent(&config)?;
            println!("{}", output.report);
            if output.run.status == RunStatus::Failure {
                std::process::exit(1);
            }
            Ok(())
        }
        Commands::Status(args) => {
            println!("{}", core_ops::cli::status::render_status(args.state_file));
            Ok(())
        }
        Commands::Explain(args) => {
            set_systemd_unit_dir(&args.systemd_unit_dir);
            set_host_override(&args.host);
            let (repo_source, revision) =
                explain_cmd::resolve_explain_target(args.repo.as_deref(), args.rev.as_deref())?;
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

fn resolve_env(value: Option<String>, key: &str) -> Result<String, CoreError> {
    if let Some(value) = value {
        return Ok(value);
    }
    if let Ok(value) = std::env::var(key) {
        if !value.is_empty() {
            return Ok(value);
        }
    }
    Err(CoreError::new(
        core_ops::core::types::FailureClass::Apply,
        format!("missing required value for {key}"),
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
