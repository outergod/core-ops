use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

const PLAN_AFTER_HELP: &str = "Examples:
  core-ops plan --repo ./repo --rev main
  core-ops plan --repo ./repo --rev main --host edge-01";

const APPLY_AFTER_HELP: &str = "Examples:
  core-ops apply --repo ./repo --rev main
  core-ops apply --repo ./repo --rev main --rollback-to rev-1
  core-ops apply --repo ./repo --rev main --rollback-to rev-1 --rollback-plan-only

Deterministic reconciliation uses desired, last_applied, and actual state.
Automatic retry is bounded; repeated failure or oscillation is surfaced in the
structured convergence output instead of retrying indefinitely.";

const STATUS_AFTER_HELP: &str = "Examples:
  core-ops status
  core-ops status --state-file /var/lib/core-ops/status.json

Status output reads the canonical persisted provenance snapshot. Deterministic
reconciliation convergence details are reported alongside the structured apply
and audit flows.";

#[derive(Parser, Debug)]
#[command(
    name = "core-ops",
    version,
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
