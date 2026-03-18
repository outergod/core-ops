use std::path::PathBuf;

use core_ops::cli::{apply as apply_cmd, plan as plan_cmd};
use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::ReconcileDependencies;
use core_ops::io::{audit as audit_io, observed, repo};

fn main() {
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
            let audit_path = audit_io::write_audit_record(&audit_dir, &output.audit_record)
                .map_err(map_plan_error)?;

            println!("{}", output.summary);
            println!("audit {}", audit_path);
            Ok(())
        }
        "apply" => {
            let (repo_path, rev, quadlet_dir, audit_dir) = parse_plan_args(args)?;

            let run = apply_cmd::apply(&repo_path, &rev, &quadlet_dir)?;
            let _ = audit_dir;

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
) -> Result<(PathBuf, String, PathBuf, PathBuf), CoreError> {
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
    let quadlet_dir = quadlet_dir.ok_or_else(|| CoreError::new(
        core_ops::core::types::FailureClass::Validation,
        "missing --quadlet-dir".to_string(),
    ))?;
    let audit_dir = audit_dir.ok_or_else(|| CoreError::new(
        core_ops::core::types::FailureClass::Validation,
        "missing --audit-dir".to_string(),
    ))?;

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

fn map_plan_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError::new(core_ops::core::types::FailureClass::Plan, err.to_string())
}
