use core_ops::cli::args::{Cli, Commands};
use core_ops::cli::common as cli_common;
use core_ops::cli::{apply as apply_cmd, plan as plan_cmd};
use core_ops::cli::agent as agent_cmd;
use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::ReconcileDependencies;
use core_ops::io::{audit as audit_io, observed, repo};
use core_ops::io::systemd::SYSTEMD_UNIT_DIR_ENV;
use log::LevelFilter;
use clap::Parser;
use std::path::PathBuf;

fn main() {
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
            set_systemd_unit_dir(&args.systemd_unit_dir);

            let deps = ReconcileDependencies {
                load_desired: &|| repo::load_desired_state(&repo_source, &rev).map_err(map_plan_error),
                read_observed: &|| {
                    observed::read_observed_state(&quadlet_dir, None).map_err(map_plan_error)
                },
                apply_plan: &|_, _| Ok(()),
            };

            let output = plan_cmd::plan(&deps)?;
            audit_io::emit_journal_event(&output.audit_event).map_err(map_plan_error)?;
            if let Some(dir) = audit_dir {
                let audit_path = audit_io::write_audit_record(&dir, &output.audit_record)
                    .map_err(map_plan_error)?;
                println!("audit {}", audit_path);
            }

            println!("{}", output.summary);
            Ok(())
        }
        Commands::Apply(args) => {
            let repo_source = args.repo;
            let rev = args.rev;
            let quadlet_dir = args.quadlet_dir;
            let audit_dir = args.audit_dir;
            let no_reload = args.no_reload;
            set_systemd_unit_dir(&args.systemd_unit_dir);

            let (result, report, plan) =
                apply_cmd::apply_with_report(&repo_source, &rev, &quadlet_dir, !no_reload)?;
            let run = result.run;
            let event = core_ops::core::audit::build_audit_event(
                &run,
                Some(&plan),
                &result.verification_results,
            );
            audit_io::emit_journal_event(&event).map_err(map_apply_error)?;
            if let Some(dir) = audit_dir {
                let record = core_ops::core::audit::build_audit_record(
                    &run.run_id,
                    Vec::new(),
                    &plan,
                    result.verification_results,
                );
                let _ = audit_io::write_audit_record(&dir, &record).map_err(map_apply_error)?;
            }

            println!("{}", report);
            println!("{}", run.summary);
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
            let audit_dir = args
                .audit_dir
                .or_else(|| std::env::var_os("CORE_OPS_AUDIT_DIR").map(PathBuf::from));
            let lock_path = args
                .lock_path
                .or_else(|| std::env::var_os("CORE_OPS_LOCK_PATH").map(PathBuf::from));

            let config = agent_cmd::AgentConfig {
                repo,
                rev,
                quadlet_dir,
                audit_dir,
                reload_systemd: !args.no_reload,
                lock_path,
            };

            let output = agent_cmd::run_agent(&config)?;
            println!("{}", output.report);
            println!("{}", output.run.summary);
            Ok(())
        }
        Commands::Status(args) => {
            let audit_file = args.audit_file;
            let contents = std::fs::read_to_string(&audit_file).map_err(map_plan_error)?;
            println!("{}", core_ops::cli::status::format_status_text(&contents));
            Ok(())
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
