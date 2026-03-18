use std::path::PathBuf;

use core_ops::cli::{apply as apply_cmd, plan as plan_cmd};
use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::ReconcileDependencies;
use core_ops::io::{audit as audit_io, observed, repo};
use log::LevelFilter;

fn main() {
    init_logging();
    if let Err(err) = run() {
        eprintln!("error: {}", err.message);
        std::process::exit(1);
    }
}

fn run() -> Result<(), CoreError> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();

    match command.as_str() {
        "plan" => {
            let (repo_path, rev, quadlet_dir, audit_dir) = parse_plan_args(args)?;

            let deps = ReconcileDependencies {
                load_desired: &|| repo::load_desired_state(&repo_path, &rev).map_err(map_plan_error),
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
        "apply" => {
            let (repo_path, rev, quadlet_dir, audit_dir) = parse_plan_args(args)?;

            let run = apply_cmd::apply(&repo_path, &rev, &quadlet_dir)?;
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
        "status" => {
            let audit_file = parse_status_args(args)?;
            let contents = std::fs::read_to_string(&audit_file).map_err(map_plan_error)?;
            println!("{}", contents.trim_end());
            Ok(())
        }
        _ => Err(CoreError::new(
            core_ops::core::types::FailureClass::Validation,
            usage(),
        )),
    }
}

fn parse_plan_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(PathBuf, String, PathBuf, Option<PathBuf>), CoreError> {
    let mut repo_path: Option<PathBuf> = None;
    let mut rev: Option<String> = None;
    let mut quadlet_dir: Option<PathBuf> = None;
    let mut audit_dir: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo" => repo_path = args.next().map(PathBuf::from),
            "--rev" => rev = args.next(),
            "--quadlet-dir" => quadlet_dir = args.next().map(PathBuf::from),
            "--audit-dir" => audit_dir = args.next().map(PathBuf::from),
            _ => {}
        }
    }

    let repo_path = repo_path.ok_or_else(|| CoreError::new(
        core_ops::core::types::FailureClass::Validation,
        "missing --repo".to_string(),
    ))?;
    let rev = rev.ok_or_else(|| CoreError::new(
        core_ops::core::types::FailureClass::Validation,
        "missing --rev".to_string(),
    ))?;
    let quadlet_dir = quadlet_dir.unwrap_or_else(|| PathBuf::from("/etc/containers/systemd"));
    Ok((repo_path, rev, quadlet_dir, audit_dir))
}

fn parse_status_args(
    mut args: impl Iterator<Item = String>,
) -> Result<PathBuf, CoreError> {
    let mut audit_file: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        if arg == "--audit-file" {
            audit_file = args.next().map(PathBuf::from);
        }
    }

    audit_file.ok_or_else(|| CoreError::new(
        core_ops::core::types::FailureClass::Validation,
        "missing --audit-file".to_string(),
    ))
}

fn usage() -> String {
    "usage: core-ops <plan|apply|status> [args]".to_string()
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
