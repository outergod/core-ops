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
  core-ops plan --source-repo ./my-repo --host edge-01

Init'd mode (default, no --source-repo): requires prior initialization
via 'core-ops init'. Repository and ref are sourced exclusively from
persisted controller configuration.

Stateless mode (--source-repo <PATH>): sources desired state from the
filesystem directory at <PATH>, bypassing the persisted controller
configuration written by 'core-ops init'. Requires --host. Writes
nothing to /var/lib/core-ops/. Honors --audit-dir when explicitly set.
For long-lived tracking, run 'core-ops init <repo> <ref>' once and omit
--source-repo on subsequent invocations.

Human-readable plan headers keep the immutable target revision primary and
render a meaningful requested ref secondarily, for example:
  454ac5f1 (demo-uat-v1) → 221145e6 (demo-uat-v2)";

const APPLY_AFTER_HELP: &str = "Examples:
  core-ops apply
  core-ops apply --json
  core-ops apply --verbose
  core-ops apply --rollback-to rev-1
  core-ops apply --rollback-to rev-1 --rollback-plan-only
  core-ops apply --source-repo ./my-repo --host edge-01

Init'd mode (default, no --source-repo): requires prior initialization
via 'core-ops init'. Repository and ref are sourced exclusively from
persisted controller configuration.

Stateless mode (--source-repo <PATH>): converges host state from the
filesystem directory at <PATH>, bypassing the persisted controller
configuration written by 'core-ops init'. Requires --host. Writes audit
records but does not mutate persisted controller state — the
init'd configuration's desired_state.* fields are preserved byte-identical.
For long-lived tracking, run 'core-ops init <repo> <ref>' once and omit
--source-repo on subsequent invocations.

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
  core-ops explain --source-repo ./my-repo --host edge-01 caddy.container

Explain output inspects a single known managed object using the authoritative
plan/result model and renders full dependency and metadata context.

Init'd mode (default, no --source-repo): requires prior initialization
via 'core-ops init'. Repository and ref are sourced exclusively from
persisted controller configuration.

Stateless mode (--source-repo <PATH>): inspects the directory at <PATH>
without consulting persisted state. Requires --host. Pure-read; writes
nothing anywhere. For long-lived tracking, run 'core-ops init <repo> <ref>'
once and omit --source-repo on subsequent invocations.";

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
    /// Manage agent skill bundles (authoring aids).
    Skill(SkillArgs),
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
    /// Use a filesystem path as the source of desired state, bypassing the
    /// persisted controller configuration written by 'core-ops init'.
    /// Requires --host. The init'd mode (no flag) sources from persisted
    /// state set by 'core-ops init <repo> <ref>'. Writes nothing under
    /// /var/lib/core-ops/. Honors --audit-dir when explicitly set.
    #[arg(long, value_name = "PATH", requires = "host")]
    pub source_repo: Option<PathBuf>,
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
    /// Use a filesystem path as the source of desired state, bypassing the
    /// persisted controller configuration written by 'core-ops init'.
    /// Requires --host. The init'd configuration's desired_state.* fields
    /// are preserved byte-identical. Audit records are written; the canonical
    /// /var/lib/core-ops/status.json is never mutated by stateless apply.
    /// For long-lived tracking, run 'core-ops init <repo> <ref>' once and
    /// omit --source-repo on subsequent invocations.
    #[arg(long, value_name = "PATH", requires = "host")]
    pub source_repo: Option<PathBuf>,
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

/// Top-level args for `core-ops skill <op>`.
#[derive(Args, Debug)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub op: SkillOp,
}

/// Operations under `core-ops skill`. Currently only `install` is
/// defined; future operations (e.g. `list`, `uninstall`) MAY land here.
#[derive(Subcommand, Debug)]
pub enum SkillOp {
    /// Install the bundled `core-ops-source-repo` agent skill.
    Install(SkillInstallArgs),
}

/// Args for `core-ops skill install`. Per
/// `specs/016-source-repository-layout/contracts/skill-cli.md`,
/// `--global` and `--print` are mutually exclusive; default mode
/// writes the bundle to `<cwd>/.agents/skills/core-ops-source-repo/`.
#[derive(Args, Debug)]
pub struct SkillInstallArgs {
    /// Write the skill bundle to `$HOME/.agents/skills/core-ops-source-repo/`
    /// instead of `<cwd>/.agents/skills/core-ops-source-repo/`.
    #[arg(long, conflicts_with = "print")]
    pub global: bool,
    /// Write the skill bundle to standard output and perform no
    /// filesystem writes. Output is a concatenation of header lines
    /// `==> <relative-path> <==` followed by each entry's bytes.
    #[arg(long, conflicts_with = "global")]
    pub print: bool,
}

#[derive(Args, Debug)]
pub struct ExplainArgs {
    /// Managed object id or display id to inspect.
    pub object: String,
    /// Host identity override for selecting hosts/<host>.
    #[arg(long)]
    pub host: Option<String>,
    /// Use a filesystem path as the source of desired state, bypassing the
    /// persisted controller configuration written by 'core-ops init'.
    /// Requires --host. Pure-read; writes nothing anywhere. For long-lived
    /// tracking, run 'core-ops init <repo> <ref>' once and omit
    /// --source-repo on subsequent invocations.
    #[arg(long, value_name = "PATH", requires = "host")]
    pub source_repo: Option<PathBuf>,
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
    use super::{Cli, Commands, GLOBAL_AFTER_HELP};
    use crate::build_info::{cli_license_notice, long_version_text};
    use clap::{CommandFactory, Parser};

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

    // ---- spec/017: --source-repo flag parsing (FR-010..FR-016) ----

    #[test]
    fn plan_accepts_source_repo_with_host() {
        let cli = Cli::try_parse_from([
            "core-ops",
            "plan",
            "--source-repo",
            "/tmp/example",
            "--host",
            "edge-01",
        ])
        .expect("plan should accept --source-repo with --host");
        match cli.command {
            Commands::Plan(args) => {
                assert_eq!(
                    args.source_repo.as_deref(),
                    Some(std::path::Path::new("/tmp/example"))
                );
                assert_eq!(args.host.as_deref(), Some("edge-01"));
            }
            _ => panic!("expected Plan subcommand"),
        }
    }

    #[test]
    fn apply_accepts_source_repo_with_host() {
        let cli = Cli::try_parse_from([
            "core-ops",
            "apply",
            "--source-repo",
            "/tmp/example",
            "--host",
            "edge-01",
        ])
        .expect("apply should accept --source-repo with --host");
        match cli.command {
            Commands::Apply(args) => {
                assert_eq!(
                    args.source_repo.as_deref(),
                    Some(std::path::Path::new("/tmp/example"))
                );
                assert_eq!(args.host.as_deref(), Some("edge-01"));
            }
            _ => panic!("expected Apply subcommand"),
        }
    }

    #[test]
    fn explain_accepts_source_repo_with_host() {
        let cli = Cli::try_parse_from([
            "core-ops",
            "explain",
            "--source-repo",
            "/tmp/example",
            "--host",
            "edge-01",
            "caddy.container",
        ])
        .expect("explain should accept --source-repo with --host");
        match cli.command {
            Commands::Explain(args) => {
                assert_eq!(
                    args.source_repo.as_deref(),
                    Some(std::path::Path::new("/tmp/example"))
                );
                assert_eq!(args.host.as_deref(), Some("edge-01"));
                assert_eq!(args.object, "caddy.container");
            }
            _ => panic!("expected Explain subcommand"),
        }
    }

    #[test]
    fn plan_source_repo_without_host_errors() {
        let err = Cli::try_parse_from([
            "core-ops",
            "plan",
            "--source-repo",
            "/tmp/example",
        ])
        .expect_err("plan --source-repo without --host should error");
        let msg = err.to_string();
        assert!(
            msg.contains("--host") || msg.contains("host"),
            "error should mention --host requirement: {msg}"
        );
    }

    #[test]
    fn apply_source_repo_without_host_errors() {
        let err = Cli::try_parse_from([
            "core-ops",
            "apply",
            "--source-repo",
            "/tmp/example",
        ])
        .expect_err("apply --source-repo without --host should error");
        let msg = err.to_string();
        assert!(
            msg.contains("--host") || msg.contains("host"),
            "error should mention --host requirement: {msg}"
        );
    }

    #[test]
    fn explain_source_repo_without_host_errors() {
        let err = Cli::try_parse_from([
            "core-ops",
            "explain",
            "--source-repo",
            "/tmp/example",
            "caddy.container",
        ])
        .expect_err("explain --source-repo without --host should error");
        let msg = err.to_string();
        assert!(
            msg.contains("--host") || msg.contains("host"),
            "error should mention --host requirement: {msg}"
        );
    }

    #[test]
    fn init_rejects_source_repo() {
        let err = Cli::try_parse_from([
            "core-ops",
            "init",
            "--source-repo",
            "/tmp/example",
            "/repo",
            "main",
        ])
        .expect_err("init must reject --source-repo");
        assert!(
            err.to_string().contains("--source-repo")
                || err.to_string().contains("unexpected"),
            "expected unexpected-argument error: {err}"
        );
    }

    #[test]
    fn agent_rejects_source_repo() {
        let err = Cli::try_parse_from([
            "core-ops",
            "agent",
            "--source-repo",
            "/tmp/example",
        ])
        .expect_err("agent must reject --source-repo");
        assert!(
            err.to_string().contains("--source-repo")
                || err.to_string().contains("unexpected"),
            "expected unexpected-argument error: {err}"
        );
    }

    #[test]
    fn status_rejects_source_repo() {
        let err = Cli::try_parse_from([
            "core-ops",
            "status",
            "--source-repo",
            "/tmp/example",
        ])
        .expect_err("status must reject --source-repo");
        assert!(
            err.to_string().contains("--source-repo")
                || err.to_string().contains("unexpected"),
            "expected unexpected-argument error: {err}"
        );
    }

    #[test]
    fn plan_help_documents_source_repo_contract() {
        let mut command = Cli::command();
        let plan_command = command.find_subcommand_mut("plan").expect("plan subcommand");
        let help = plan_command.render_long_help().to_string();
        assert!(help.contains("--source-repo"), "plan --help missing --source-repo");
        assert!(help.contains("--host"), "plan --help missing --host requirement");
        assert!(
            help.contains("init"),
            "plan --help missing init pointer per FR-016 contract"
        );
    }

    #[test]
    fn apply_help_documents_source_repo_contract() {
        let mut command = Cli::command();
        let apply_command = command.find_subcommand_mut("apply").expect("apply subcommand");
        let help = apply_command.render_long_help().to_string();
        assert!(help.contains("--source-repo"), "apply --help missing --source-repo");
        assert!(help.contains("--host"), "apply --help missing --host requirement");
        assert!(
            help.contains("init"),
            "apply --help missing init pointer per FR-016 contract"
        );
    }

    #[test]
    fn explain_help_documents_source_repo_contract() {
        let mut command = Cli::command();
        let explain_command = command
            .find_subcommand_mut("explain")
            .expect("explain subcommand");
        let help = explain_command.render_long_help().to_string();
        assert!(help.contains("--source-repo"), "explain --help missing --source-repo");
        assert!(help.contains("--host"), "explain --help missing --host requirement");
        assert!(
            help.contains("init"),
            "explain --help missing init pointer per FR-016 contract"
        );
    }
}
