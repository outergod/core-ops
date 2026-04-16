use std::path::PathBuf;

use crate::build_info::long_version_text;
use clap::{Args, Parser, Subcommand};

const INIT_AFTER_HELP: &str = "Examples:
  core-ops init /path/to/repo main
  core-ops init https://git.example.com/repo.git main
  core-ops init /path/to/repo v1.0.0
  core-ops init /path/to/repo main --force

Run this command once before using plan, apply, agent, or explain.
Use --force to overwrite existing configuration or recover from a corrupt state file.";

const PLAN_AFTER_HELP: &str = "Examples:
  core-ops plan
  core-ops plan --host edge-01

Requires prior initialization via 'core-ops init'. Repository and ref are
sourced exclusively from persisted controller configuration.

Human-readable plan headers keep the immutable target revision primary and
render a meaningful requested ref secondarily, for example:
  454ac5f1 (demo-uat-v1) → 221145e6 (demo-uat-v2)";

const APPLY_AFTER_HELP: &str = "Examples:
  core-ops apply
  core-ops apply --json
  core-ops apply --verbose
  core-ops apply --rollback-to rev-1
  core-ops apply --rollback-to rev-1 --rollback-plan-only

Requires prior initialization via 'core-ops init'. Repository and ref are
sourced exclusively from persisted controller configuration.

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
  core-ops explain mount/var-lib-demo.mount --json

Explain output inspects a single known managed object using the authoritative
plan/result model and renders full dependency and metadata context.

Requires prior initialization via 'core-ops init'. Repository and ref are
sourced exclusively from persisted controller configuration.";

const GLOBAL_AFTER_HELP: &str = "License:
  GNU Affero General Public License version 3 or later (AGPLv3+)";

#[derive(Parser, Debug)]
#[command(
    name = "core-ops",
    version = long_version_text(),
    long_version = long_version_text(),
    about = "GitOps controller for Quadlet, native systemd units, and mount-aware reconciliation",
    after_help = GLOBAL_AFTER_HELP
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize the controller with a source repository and tracking ref.
    #[command(after_help = INIT_AFTER_HELP)]
    Init(InitArgs),
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
pub struct InitArgs {
    /// Source repository (local path or Git URL).
    pub repository: String,
    /// Branch or tag name to track.
    pub requested_ref: String,
    /// Overwrite existing configuration; required when re-attaching from Detached state.
    #[arg(long)]
    pub force: bool,
    /// Optional path to the canonical persisted provenance status file.
    #[arg(long)]
    pub state_file: Option<std::path::PathBuf>,
}

#[derive(Args, Debug)]
pub struct PlanArgs {
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

#[cfg(test)]
mod tests {
    use super::{Cli, GLOBAL_AFTER_HELP};
    use crate::build_info::{cli_license_notice, long_version_text};
    use clap::CommandFactory;

    #[test]
    fn long_version_includes_package_version() {
        assert!(long_version_text().contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn short_and_long_version_flags_share_the_same_rendered_version() {
        let command = Cli::command();

        assert_eq!(command.get_version(), Some(long_version_text()));
        assert_eq!(command.get_long_version(), Some(long_version_text()));
    }

    #[test]
    fn help_mentions_governing_license() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        assert!(help.contains(cli_license_notice()));
        assert!(help.contains(GLOBAL_AFTER_HELP));
    }
}
