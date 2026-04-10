use clap::{Args, Parser, Subcommand};
use core_ops::build_info::long_version_text;
use core_ops::cli::common as cli_common;
use core_ops::cli::report::{
    format_release_governance_changelog_report, format_release_governance_json,
    format_release_governance_report,
};
use core_ops::core::errors::CoreError;
use core_ops::core::release_governance::{
    evaluate_release_governance, render_generated_changelog, GovernanceStatus,
};
use core_ops::core::types::FailureClass;
use core_ops::io::release_governance::{load_governance_repository_input, load_release_fragments};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "core-ops-release",
    version = long_version_text(),
    long_version = long_version_text(),
    about = "Dedicated release-governance entrypoint for CoreOps maintainers and CI"
)]
pub struct ReleaseCli {
    #[command(subcommand)]
    pub command: ReleaseCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ReleaseCommands {
    Validate(ValidateArgs),
    Changelog(ChangelogArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ValidateArgs {
    #[arg(long, default_value = ".")]
    pub repo_root: PathBuf,
    #[arg(long)]
    pub base_ref: Option<String>,
    #[arg(long)]
    pub head_ref: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ChangelogArgs {
    #[arg(long, default_value = ".")]
    pub repo_root: PathBuf,
    #[arg(long, default_value = "CHANGELOG.md")]
    pub output: PathBuf,
    #[arg(long)]
    pub check: bool,
    #[arg(long)]
    pub write: bool,
}

fn main() {
    let cli = ReleaseCli::parse();
    if let Err(err) = run_and_exit(cli) {
        let report = cli_common::report_error(err);
        eprintln!("{:?}", report);
        std::process::exit(1);
    }
}

fn run_and_exit(cli: ReleaseCli) -> Result<(), CoreError> {
    match cli.command {
        ReleaseCommands::Validate(args) => {
            let input = load_governance_repository_input(
                &args.repo_root,
                args.base_ref.as_deref(),
                args.head_ref.as_deref(),
            )?;
            let result = evaluate_release_governance(&input)?;
            if args.json {
                println!("{}", format_release_governance_json(&result));
            } else {
                println!("{}", format_release_governance_report(&result));
            }
            if result.overall_status == GovernanceStatus::Failed {
                std::process::exit(1);
            }
        }
        ReleaseCommands::Changelog(args) => {
            let existing = fs::read_to_string(args.repo_root.join(&args.output)).map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!("failed to read {}: {err}", args.output.display()),
                )
            })?;
            let fragments = load_release_fragments(&args.repo_root)?;
            let rendered = render_generated_changelog(&existing, &fragments)?;
            if args.check {
                let aligned = rendered == existing;
                println!(
                    "{}",
                    format_release_governance_changelog_report(&args.output, aligned, false)
                );
                if !aligned {
                    std::process::exit(1);
                }
            } else if args.write {
                fs::write(args.repo_root.join(&args.output), rendered).map_err(|err| {
                    CoreError::new(
                        FailureClass::Apply,
                        format!("failed to write {}: {err}", args.output.display()),
                    )
                })?;
                println!(
                    "{}",
                    format_release_governance_changelog_report(&args.output, true, true)
                );
            } else {
                print!("{rendered}");
            }
        }
    }
    Ok(())
}
