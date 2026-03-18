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
    /// Display a stored audit record.
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
    /// System-level Quadlet directory.
    #[arg(long, default_value = "/etc/containers/systemd")]
    pub quadlet_dir: PathBuf,
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
    /// System-level Quadlet directory.
    #[arg(long, default_value = "/etc/containers/systemd")]
    pub quadlet_dir: PathBuf,
    /// Optional directory for persisted audit records.
    #[arg(long)]
    pub audit_dir: Option<PathBuf>,
    /// Skip systemd daemon-reload after applying changes.
    #[arg(long)]
    pub no_reload: bool,
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Path to an audit record file.
    #[arg(long)]
    pub audit_file: PathBuf,
}
