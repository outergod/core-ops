use core_ops::cli::args::{Cli, Commands};
use core_ops::cli::common as cli_common;
use core_ops::cli::{apply as apply_cmd, plan as plan_cmd};
use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::ReconcileDependencies;
use core_ops::io::{audit as audit_io, observed, repo};
use log::LevelFilter;
use clap::Parser;

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

            let run = apply_cmd::apply(&repo_source, &rev, &quadlet_dir, !no_reload)?;
            let event = core_ops::core::audit::build_audit_event(&run, None);
            audit_io::emit_journal_event(&event).map_err(map_apply_error)?;
            if let Some(dir) = audit_dir {
                let record = core_ops::core::audit::build_audit_record(
                    &run.run_id,
                    Vec::new(),
                    &core_ops::core::types::ReconciliationPlan {
                        plan_id: "apply".to_string(),
                        desired_revision_id: rev.clone(),
                        observed_revision_id: None,
                        actions: Vec::new(),
                        safety_checks: Vec::new(),
                        expected_outcomes: Vec::new(),
                    },
                );
                let _ = audit_io::write_audit_record(&dir, &record).map_err(map_apply_error)?;
            }

            println!("{}", run.summary);
            Ok(())
        }
        Commands::Status(args) => {
            let audit_file = args.audit_file;
            let contents = std::fs::read_to_string(&audit_file).map_err(map_plan_error)?;
            println!("{}", contents.trim_end());
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
