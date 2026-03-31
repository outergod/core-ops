use std::path::PathBuf;
use std::sync::OnceLock;

use crate::build_info::{BUILD_REVISION, BUILD_TREE_STATE};
use clap::{Args, Parser, Subcommand};

const PLAN_AFTER_HELP: &str = "Examples:
  core-ops plan --repo ./repo --rev main
  core-ops plan --repo ./repo --rev main --host edge-01

Human-readable plan headers keep the immutable target revision primary and
render a meaningful requested ref secondarily, for example:
  454ac5f1 (demo-uat-v1) → 221145e6 (demo-uat-v2)";

const APPLY_AFTER_HELP: &str = "Examples:
  core-ops apply --repo ./repo --rev main
  core-ops apply --repo ./repo --rev main --json
  core-ops apply --repo ./repo --rev main --verbose
  core-ops apply --repo ./repo --rev main --rollback-to rev-1
  core-ops apply --repo ./repo --rev main --rollback-to rev-1 --rollback-plan-only

Deterministic reconciliation uses desired, last_applied, and actual state.
Automatic retry is bounded; repeated failure or oscillation is surfaced in the
structured convergence output instead of retrying indefinitely.

Default human summaries contain only counts and overall outcome. Managed
transition headers keep immutable revisions primary and render meaningful
current/prior requested refs secondarily when available.";

const STATUS_AFTER_HELP: &str = "Examples:
  core-ops status
  core-ops status --state-file /var/lib/core-ops/status.json

Status output reads the canonical persisted provenance snapshot. Deterministic
reconciliation convergence details are reported alongside the structured apply
and audit flows.";

const EXPLAIN_AFTER_HELP: &str = "Examples:
  core-ops explain container/frontend.container
  core-ops explain container/frontend.container --repo ./repo
  core-ops explain mount/var-lib-demo.mount --repo ./repo --rev main --json

Explain output inspects a single known managed object using the authoritative
plan/result model and renders full dependency and metadata context.

When `--repo` and `--rev` are omitted, explain defaults to the currently
deployed target from persisted state.";

#[derive(Parser, Debug)]
#[command(
    name = "core-ops",
    version = long_version(),
    long_version = long_version(),
    about = "GitOps controller for Quadlet, native systemd units, and mount-aware reconciliation"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Compute a reconciliation plan, including native .mount/.automount artifacts with minimal [X-CoreOps] metadata and generated dependency semantics.
    #[command(after_help = PLAN_AFTER_HELP)]
    Plan(PlanArgs),
    /// Apply a reconciliation plan, including CreateMountpoint-driven mountpoint preparation and mount-aware native unit activation.
    #[command(after_help = APPLY_AFTER_HELP)]
    Apply(ApplyArgs),
    /// Run the agent once (intended for systemd service execution).
    Agent(AgentArgs),
    /// Display canonical persisted provenance from a status snapshot, treating invalid or missing snapshots as absent.
    #[command(after_help = STATUS_AFTER_HELP)]
    Status(StatusArgs),
    /// Explain a single managed object using the authoritative reconciliation model.
    #[command(after_help = EXPLAIN_AFTER_HELP)]
    Explain(ExplainArgs),
}

#[derive(Args, Debug)]
pub struct PlanArgs {
    /// Source repository (local path or Git URL).
    #[arg(long)]
    pub repo: String,
    /// Git revision (branch, tag, or commit).
    #[arg(long)]
    pub rev: String,
    /// Host identity override for selecting hosts/<host>, including host-specific mount overrides.
    #[arg(long)]
    pub host: Option<String>,
    /// System-level Quadlet directory.
    #[arg(long, default_value = "/etc/containers/systemd")]
    pub quadlet_dir: PathBuf,
    /// Systemd unit directory (defaults to /etc/systemd/system).
    #[arg(long)]
    pub systemd_unit_dir: Option<PathBuf>,
    /// Optional directory for persisted audit records.
    #[arg(long)]
    pub audit_dir: Option<PathBuf>,
    /// Emit authoritative machine-readable `PlanOutput` JSON.
    #[arg(long)]
    pub json: bool,
    /// Show full diff hunks and include zero-value summary counts.
    #[arg(long, short = 'v', conflicts_with = "json")]
    pub verbose: bool,
}

#[derive(Args, Debug)]
pub struct ApplyArgs {
    /// Source repository (local path or Git URL).
    #[arg(long)]
    pub repo: String,
    /// Git revision (branch, tag, or commit).
    #[arg(long)]
    pub rev: String,
    /// Host identity override for selecting hosts/<host>, including host-specific mount overrides.
    #[arg(long)]
    pub host: Option<String>,
    /// System-level Quadlet directory.
    #[arg(long, default_value = "/etc/containers/systemd")]
    pub quadlet_dir: PathBuf,
    /// Systemd unit directory (defaults to /etc/systemd/system).
    #[arg(long)]
    pub systemd_unit_dir: Option<PathBuf>,
    /// Optional directory for persisted audit records.
    #[arg(long)]
    pub audit_dir: Option<PathBuf>,
    /// Optional path to the canonical persisted provenance status file.
    /// When omitted, the runtime uses CORE_OPS_STATE_FILE if set, otherwise
    /// `/var/lib/core-ops/status.json`.
    #[arg(long)]
    pub state_file: Option<PathBuf>,
    /// Force apply without updating the canonical persisted provenance state, even for mount-aware reconciliation.
    #[arg(long, conflicts_with = "state_file")]
    pub force_no_state: bool,
    /// Skip systemd daemon-reload after applying changes.
    #[arg(long)]
    pub no_reload: bool,
    /// Roll back to a previously retained successful revision.
    #[arg(long)]
    pub rollback_to: Option<String>,
    /// Compute the rollback plan without executing side effects.
    #[arg(long, requires = "rollback_to")]
    pub rollback_plan_only: bool,
    /// Emit authoritative machine-readable `ApplyOutput` JSON.
    #[arg(long, conflicts_with = "verbose")]
    pub json: bool,
    /// Show phases and expanded diagnostics in human-readable output.
    #[arg(long, short = 'v', visible_alias = "debug", conflicts_with = "json")]
    pub verbose: bool,
}

#[derive(Args, Debug)]
pub struct AgentArgs {
    /// Source repository (local path or Git URL).
    #[arg(long)]
    pub repo: Option<String>,
    /// Git revision (branch, tag, or commit).
    #[arg(long)]
    pub rev: Option<String>,
    /// Host identity override for selecting hosts/<host>, including host-specific mount overrides.
    #[arg(long)]
    pub host: Option<String>,
    /// System-level Quadlet directory.
    #[arg(long)]
    pub quadlet_dir: Option<PathBuf>,
    /// Systemd unit directory (defaults to /etc/systemd/system).
    #[arg(long)]
    pub systemd_unit_dir: Option<PathBuf>,
    /// Optional directory for persisted audit records.
    #[arg(long)]
    pub audit_dir: Option<PathBuf>,
    /// Optional path to the canonical persisted provenance status file.
    /// When omitted, the runtime uses CORE_OPS_STATE_FILE if set, otherwise
    /// `/var/lib/core-ops/status.json`.
    #[arg(long)]
    pub state_file: Option<PathBuf>,
    /// Path to the run lock file.
    #[arg(long)]
    pub lock_path: Option<PathBuf>,
    /// Skip systemd daemon-reload after applying changes.
    #[arg(long)]
    pub no_reload: bool,
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Optional path to the canonical provenance status snapshot to read.
    /// When omitted, the runtime uses CORE_OPS_STATE_FILE if set, otherwise
    /// `/var/lib/core-ops/status.json`.
    #[arg(long)]
    pub state_file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ExplainArgs {
    /// Managed object id or display id to inspect.
    pub object: String,
    /// Source repository (local path or Git URL). Defaults to the currently deployed target from persisted state.
    #[arg(long)]
    pub repo: Option<String>,
    /// Git revision (branch, tag, or commit). Defaults to the currently deployed target from persisted state.
    #[arg(long)]
    pub rev: Option<String>,
    /// Host identity override for selecting hosts/<host>.
    #[arg(long)]
    pub host: Option<String>,
    /// System-level Quadlet directory.
    #[arg(long, default_value = "/etc/containers/systemd")]
    pub quadlet_dir: PathBuf,
    /// Systemd unit directory (defaults to /etc/systemd/system).
    #[arg(long)]
    pub systemd_unit_dir: Option<PathBuf>,
    /// Emit authoritative machine-readable `ExplainOutput` JSON.
    #[arg(long)]
    pub json: bool,
}

fn long_version() -> &'static str {
    static LONG_VERSION: OnceLock<String> = OnceLock::new();
    LONG_VERSION.get_or_init(|| {
        let mut version = env!("CARGO_PKG_VERSION").to_string();
        if let Some(revision) = BUILD_REVISION {
            version.push_str(&format!(" ({})", short_revision(revision)));
            if BUILD_TREE_STATE != "clean" {
                version.push_str(&format!(" {BUILD_TREE_STATE}"));
            }
        } else if BUILD_TREE_STATE != "clean" {
            version.push_str(&format!(" ({BUILD_TREE_STATE})"));
        }
        version
    })
}

fn short_revision(revision: &str) -> &str {
    &revision[..revision.len().min(8)]
}

#[cfg(test)]
mod tests {
    use super::{long_version, Cli};
    use clap::CommandFactory;

    #[test]
    fn long_version_includes_package_version() {
        assert!(long_version().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn short_and_long_version_flags_share_the_same_rendered_version() {
        let command = Cli::command();

        assert_eq!(command.get_version(), Some(long_version()));
        assert_eq!(command.get_long_version(), Some(long_version()));
    }
}
