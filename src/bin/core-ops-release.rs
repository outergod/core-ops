use clap::{Args, Parser, Subcommand};
use core_ops::build_info::long_version_text;
use core_ops::cli::common as cli_common;
use core_ops::cli::report::{
    format_release_governance_changelog_report, format_release_governance_json,
    format_release_governance_report,
};
use core_ops::core::errors::CoreError;
use core_ops::core::release_governance::{
    evaluate_release_governance, promote_changelog, render_generated_changelog, GovernanceStatus,
};
use core_ops::core::types::FailureClass;
use core_ops::io::release_governance::{load_governance_repository_input, load_release_fragments};
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

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
    Promote(PromoteArgs),
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

/// Promote `[Unreleased]` → `[<version>] - <date>` and consume the
/// fragments under `changes/` that fed it. Idempotent — running this
/// twice with the same `--version` is a no-op on the second pass.
/// Pipeline-only: humans bumping `Cargo.toml` + populating
/// `[Unreleased]` should let CI run this for them.
#[derive(Args, Debug, Clone)]
pub struct PromoteArgs {
    #[arg(long, default_value = ".")]
    pub repo_root: PathBuf,
    #[arg(long, default_value = "CHANGELOG.md")]
    pub output: PathBuf,
    /// Version to promote to, e.g. `2.0.1`. Must match the literal
    /// section heading the post-merge release pipeline grep's for
    /// (`## [<version>]`).
    #[arg(long)]
    pub version: String,
    /// Override the release date (YYYY-MM-DD). Defaults to today
    /// (UTC) at promote time.
    #[arg(long)]
    pub date: Option<String>,
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
        ReleaseCommands::Promote(args) => {
            let changelog_path = args.repo_root.join(&args.output);
            let existing = fs::read_to_string(&changelog_path).map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!("failed to read {}: {err}", changelog_path.display()),
                )
            })?;
            let date = args
                .date
                .clone()
                .map(Ok)
                .unwrap_or_else(today_utc_iso)?;
            let promoted = promote_changelog(&existing, &args.version, &date)?;
            let changelog_changed = promoted != existing;
            if changelog_changed {
                fs::write(&changelog_path, promoted).map_err(|err| {
                    CoreError::new(
                        FailureClass::Apply,
                        format!("failed to write {}: {err}", changelog_path.display()),
                    )
                })?;
            }
            // Always sweep fragments, even when the CHANGELOG already
            // has [version]. Otherwise a partial prior run that
            // promoted but failed to clean up leaves stale fragments
            // forever — self-heal must remove them on the next push.
            let removed = remove_consumed_fragments(&args.repo_root.join("changes"))?;
            if changelog_changed {
                println!(
                    "Promoted {} to [{}] - {}; removed {} consumed fragment(s).",
                    args.output.display(),
                    args.version,
                    date,
                    removed
                );
            } else {
                println!(
                    "{} already contains [{}] — no-op; removed {} stale fragment(s).",
                    args.output.display(),
                    args.version,
                    removed
                );
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

fn today_utc_iso() -> Result<String, CoreError> {
    let today = OffsetDateTime::now_utc().date();
    let format = time::format_description::parse("[year]-[month]-[day]").map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to build date format: {err}"),
        )
    })?;
    today.format(&format).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to render today's date: {err}"),
        )
    })
}

fn remove_consumed_fragments(changes_dir: &Path) -> Result<usize, CoreError> {
    if !changes_dir.exists() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(changes_dir).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to read {}: {err}", changes_dir.display()),
        )
    })? {
        let entry = entry.map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to read entry under {}: {err}", changes_dir.display()),
            )
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => continue,
        };
        if name == "README.md" {
            continue;
        }
        if !name.ends_with(".md") {
            continue;
        }
        fs::remove_file(&path).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to remove {}: {err}", path.display()),
            )
        })?;
        removed += 1;
    }
    Ok(removed)
}
