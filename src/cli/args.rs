use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "core-ops", version, about = "GitOps Quadlet controller")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Compute a reconciliation plan without applying changes.
    Plan(PlanArgs),
    /// Apply a reconciliation plan to the host.
    Apply(ApplyArgs),
    /// Run the agent once (intended for systemd service execution).
    Agent(AgentArgs),
    /// Display canonical persisted provenance from a status snapshot.
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
    /// Host identity override for selecting hosts/<host>.
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
    /// Host identity override for selecting hosts/<host>.
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
    /// When omitted, the runtime uses CORE_OPS_STATE_FILE if set.
    #[arg(long)]
    pub state_file: Option<PathBuf>,
    /// Skip systemd daemon-reload after applying changes.
    #[arg(long)]
    pub no_reload: bool,
}

#[derive(Args, Debug)]
pub struct AgentArgs {
    /// Source repository (local path or Git URL).
    #[arg(long)]
    pub repo: Option<String>,
    /// Git revision (branch, tag, or commit).
    #[arg(long)]
    pub rev: Option<String>,
    /// Host identity override for selecting hosts/<host>.
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
    /// When omitted, the runtime uses CORE_OPS_STATE_FILE if set.
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
    /// Path to the canonical provenance status snapshot to read.
    #[arg(long)]
    pub state_file: PathBuf,
}
