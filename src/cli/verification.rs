use crate::cli::report::{
    format_verification_coverage_report, format_verification_run_json,
    format_verification_run_report, format_verification_suite_report,
};
use crate::core::boundaries::{
    VerificationArtifactBoundary, VerificationGuestBoundary, VerificationLibvirtBoundary,
};
use crate::core::errors::CoreError;
use crate::core::types::{
    FailureClass, VerificationRunMode, VerificationRunOutcome, VerificationStepStatus,
};
use crate::core::verification_eval::{
    build_execution_plan, classify_run_outcome, classify_scenario_outcome, evaluate_assertions,
    parse_timeout_literal,
};
use crate::core::verification_generate::{
    build_coverage_report, generate_candidates_from_spec, load_accepted_corpus,
    render_candidate_yaml,
};
use crate::core::verification_model::{
    load_scenario_definition, VerificationAssertionResult, VerificationRevisionSelectionBasis,
    VerificationRun, VerificationRunView, VerificationRuntimeBindings,
    VerificationScenarioDefinition, VerificationScenarioOutcome, VerificationStepResult,
    VerificationStepType,
};
use crate::io::guest::GuestCommandRunner;
use crate::io::libvirt::LibvirtCommandRunner;
use crate::io::verification_artifacts::{write_diagnostic_artifacts, ArtifactCollector};
use clap::{Args, Parser, Subcommand};
use std::io::{self, Write};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const VERIFY_AFTER_HELP: &str = "Examples:
  core-ops-verify run --scenario tests/fixtures/verification/scenarios/minimal-accepted.yaml
  core-ops-verify run --scenario tests/fixtures/verification/scenarios/minimal-accepted.yaml --debug
  core-ops-verify run --scenario tests/fixtures/verification/scenarios/minimal-accepted.yaml --debug --pause-before-teardown
  core-ops-verify run --scenario tests/fixtures/verification/scenarios/minimal-accepted.yaml --verbose

Verification runs execute declarative single-VM scenarios against disposable
guests, collect offline-diagnosable artifacts, and tear the environment down
by default unless debug retention is explicitly requested.";

pub struct VerificationCommandOutput {
    pub human_report: String,
    pub machine_report: String,
    pub emit_json: bool,
    pub exit_code: i32,
}

pub enum VerificationCommandResult {
    Run(Box<VerificationCommandOutput>),
    Generate {
        human_report: String,
        exit_code: i32,
    },
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "core-ops-verify",
    about = "Dedicated end-to-end verification entrypoint for CoreOps development and CI"
)]
pub struct VerifyCli {
    #[command(subcommand)]
    pub command: VerifyCommands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum VerifyCommands {
    #[command(after_help = VERIFY_AFTER_HELP)]
    Run(VerifyRunArgs),
    Generate(VerifyGenerateArgs),
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "core-ops-verify run",
    about = "Execute a declarative verification scenario",
    after_help = VERIFY_AFTER_HELP
)]
pub struct VerifyRunArgs {
    /// Path to the declarative scenario definition to execute.
    #[arg(long)]
    pub scenario: Option<std::path::PathBuf>,
    /// Directory containing accepted scenario YAML files for CI/corpus runs.
    #[arg(long)]
    pub accepted_dir: Option<std::path::PathBuf>,
    /// Run only the selected accepted scenario IDs from `--accepted-dir`.
    #[arg(long = "scenario-id")]
    pub scenario_ids: Vec<String>,
    /// Workspace root for disposable guest state.
    #[arg(long)]
    pub workspace_root: Option<std::path::PathBuf>,
    /// Artifact output root for retained run bundles.
    #[arg(long)]
    pub artifacts_dir: Option<std::path::PathBuf>,
    /// Keep the disposable environment after artifact capture when the scenario policy allows it.
    #[arg(long)]
    pub debug: bool,
    /// Pause after artifact capture and wait for user acknowledgement before tearing the guest down.
    #[arg(long)]
    pub pause_before_teardown: bool,
    /// Use the deterministic internal synthetic boundary path instead of the authoritative VM-backed path.
    #[arg(long, hide = true)]
    pub synthetic: bool,
    /// Run in CI/non-interactive mode.
    #[arg(long)]
    pub ci: bool,
    /// Emit authoritative machine-readable `verification_run` JSON.
    #[arg(long)]
    pub json: bool,
    /// Print effective runtime, libvirt, and command details before execution.
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

#[derive(Args, Debug, Clone)]
pub struct VerifyGenerateArgs {
    /// Path to the feature specification used as generation input.
    #[arg(long)]
    pub spec: std::path::PathBuf,
    /// Directory containing accepted scenario YAML files.
    #[arg(long)]
    pub accepted_dir: std::path::PathBuf,
    /// Optional directory to write generated candidate YAML files into.
    #[arg(long)]
    pub output_dir: Option<std::path::PathBuf>,
}

pub struct VerificationExecutionContext<'a> {
    pub workspace: &'a Path,
    pub artifacts_root: &'a Path,
    pub libvirt: &'a dyn VerificationLibvirtBoundary,
    pub guest_boundary: &'a dyn VerificationGuestBoundary,
    pub artifact_boundary: &'a dyn VerificationArtifactBoundary,
}

struct VerificationSuiteJsonView<'a> {
    run_id: &'a str,
    mode: VerificationRunMode,
    overall_outcome: VerificationRunOutcome,
    revision_under_test: &'a str,
    started_at: &'a str,
    completed_at: &'a str,
    bundle_path: &'a str,
    environment_retained: bool,
    scenario_outcomes: &'a [VerificationScenarioOutcome],
}

pub fn run(args: &VerifyRunArgs) -> Result<VerificationCommandOutput, CoreError> {
    validate_run_args(args)?;
    let mode = if args.debug {
        VerificationRunMode::Debug
    } else if args.ci {
        VerificationRunMode::Ci
    } else {
        VerificationRunMode::Local
    };
    let workspace_root = args
        .workspace_root
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("core-ops-verification"));
    let artifacts_root = args
        .artifacts_dir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("core-ops-verification-artifacts"));

    let libvirt = LibvirtCommandRunner::from_env(!args.synthetic);
    let guest_boundary = GuestCommandRunner::default();
    let artifact_boundary = ArtifactCollector;
    match (&args.scenario, &args.accepted_dir) {
        (Some(scenario_path), None) => {
            let scenario = load_scenario_definition(scenario_path)?;
            let run_id = next_run_id(&scenario.scenario_id);
            let workspace = workspace_root.join(&run_id);
            if args.verbose {
                eprintln!(
                    "{}",
                    render_verbose_run_context(&scenario, &run_id, &libvirt, &guest_boundary)
                );
            }
            let context = VerificationExecutionContext {
                workspace: &workspace,
                artifacts_root: &artifacts_root,
                libvirt: &libvirt,
                guest_boundary: &guest_boundary,
                artifact_boundary: &artifact_boundary,
            };

            let view = execute_scenario(
                &scenario,
                mode,
                &run_id,
                &context,
                args.verbose,
                args.pause_before_teardown,
            )?;
            let exit_code = if view.overall_outcome == VerificationRunOutcome::Passed {
                0
            } else {
                1
            };
            Ok(VerificationCommandOutput {
                human_report: format_verification_run_report(&view),
                machine_report: format_verification_run_json(&view),
                emit_json: args.json,
                exit_code,
            })
        }
        (None, Some(accepted_dir)) => run_accepted_corpus(
            args,
            accepted_dir,
            mode,
            &workspace_root,
            &artifacts_root,
            &libvirt,
            &guest_boundary,
        ),
        (Some(_), Some(_)) => Err(CoreError::new(
            FailureClass::Validation,
            "use either --scenario or --accepted-dir, not both",
        )),
        (None, None) => Err(CoreError::new(
            FailureClass::Validation,
            "one of --scenario or --accepted-dir is required",
        )),
    }
}

fn run_accepted_corpus(
    args: &VerifyRunArgs,
    accepted_dir: &Path,
    mode: VerificationRunMode,
    workspace_root: &Path,
    artifacts_root: &Path,
    libvirt: &LibvirtCommandRunner,
    guest_boundary: &GuestCommandRunner,
) -> Result<VerificationCommandOutput, CoreError> {
    let mut scenarios = load_accepted_corpus(accepted_dir)?;
    if !args.scenario_ids.is_empty() {
        scenarios.retain(|scenario| args.scenario_ids.iter().any(|id| id == &scenario.scenario_id));
    }
    if scenarios.is_empty() {
        return Err(CoreError::new(
            FailureClass::Validation,
            "no accepted scenarios matched the requested corpus selection",
        ));
    }

    let run_id = next_run_id("accepted-corpus");
    let suite_workspace_root = workspace_root.join(&run_id);
    let suite_artifacts_root = artifacts_root.join(&run_id);
    std::fs::create_dir_all(&suite_workspace_root).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create suite workspace root {}: {err}",
                suite_workspace_root.display()
            ),
        )
    })?;
    std::fs::create_dir_all(&suite_artifacts_root).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create suite artifacts root {}: {err}",
                suite_artifacts_root.display()
            ),
        )
    })?;

    let started_at = now_rfc3339()?;
    let mut views = Vec::new();
    let artifact_boundary = ArtifactCollector;
    for scenario in &scenarios {
        let scenario_run_id = format!("{run_id}-{}", scenario.scenario_id);
        let workspace = suite_workspace_root.join(&scenario.scenario_id);
        if args.verbose {
            eprintln!(
                "{}",
                render_verbose_run_context(scenario, &scenario_run_id, libvirt, guest_boundary)
            );
        }
        let context = VerificationExecutionContext {
            workspace: &workspace,
            artifacts_root: &suite_artifacts_root,
            libvirt,
            guest_boundary,
            artifact_boundary: &artifact_boundary,
        };
        views.push(execute_scenario(
            scenario,
            mode,
            &scenario_run_id,
            &context,
            args.verbose,
            false,
        )?);
    }
    let completed_at = now_rfc3339()?;

    let scenario_outcomes = views
        .iter()
        .map(|view| VerificationScenarioOutcome {
            scenario_id: view.scenario_id.clone(),
            revision_under_test: view.revision_under_test.clone(),
            outcome: view.overall_outcome,
            step_results: view.step_results.clone(),
            assertion_results: view.assertion_results.clone(),
            failure_summary: view.failure_summary.clone(),
        })
        .collect::<Vec<_>>();
    let overall_outcome = classify_run_outcome(&scenario_outcomes);
    let suite_bundle_path = suite_artifacts_root.display().to_string();
    let suite_revision_under_test = summarize_suite_revision_under_test(&scenario_outcomes);
    let suite_environment_retained = views.iter().any(|view| view.environment_retained);
    write_suite_bundle_index(&suite_bundle_path, &views)?;

    let human_report = format_verification_suite_report(
        &run_id,
        mode,
        overall_outcome,
        &suite_revision_under_test,
        &scenario_outcomes,
        &suite_bundle_path,
    );
    let machine_report = format_verification_suite_json(&VerificationSuiteJsonView {
        run_id: &run_id,
        mode,
        overall_outcome,
        revision_under_test: &suite_revision_under_test,
        started_at: &started_at,
        completed_at: &completed_at,
        bundle_path: &suite_bundle_path,
        environment_retained: suite_environment_retained,
        scenario_outcomes: &scenario_outcomes,
    });
    let exit_code = if overall_outcome == VerificationRunOutcome::Passed {
        0
    } else {
        1
    };

    Ok(VerificationCommandOutput {
        human_report,
        machine_report,
        emit_json: args.json,
        exit_code,
    })
}

fn render_verbose_run_context(
    scenario: &VerificationScenarioDefinition,
    run_id: &str,
    libvirt: &LibvirtCommandRunner,
    guest: &GuestCommandRunner,
) -> String {
    let environment = scenario.effective_environment().ok();
    let timeouts = scenario.effective_timeouts().ok();
    let mut lines = vec![
        "Verification Debug".to_string(),
        "──────────────────".to_string(),
        format!("scenario: {}", scenario.scenario_id),
        format!("run_id: {}", run_id),
        format!("vm_backed: {}", libvirt.env_backed),
        format!("network_mode: {}", libvirt.network_mode),
        format!("libvirt_uri: {}", libvirt.connection_uri),
        format!(
            "vm_host: {}",
            libvirt.vm_host.as_deref().unwrap_or("<local>")
        ),
        format!("ssh_binary: {}", guest.ssh_binary),
        format!("scp_binary: {}", guest.scp_binary),
    ];

    if let Some(environment) = environment {
        lines.push(format!("environment_profile: {}", scenario.environment.profile));
        lines.push(format!("guest_name: {}", environment.guest.guest_name));
        lines.push(format!("guest_image_version: {}", environment.image_version));
    }
    if let Some(timeouts) = timeouts {
        lines.push(format!("scenario_timeout: {}", timeouts.scenario_timeout));
        lines.push(format!("readiness_timeout: {}", timeouts.readiness_timeout));
    }
    if libvirt.env_backed {
        if let Some(interface) = &libvirt.network_interface {
            lines.push(format!("network_interface: {interface}"));
        }
        if let Some(subnet_cidr) = &libvirt.subnet_cidr {
            lines.push(format!("subnet_cidr: {subnet_cidr}"));
        }
        if let Some(ip_pool) = &libvirt.ip_pool {
            lines.push(format!("ip_pool: {ip_pool}"));
        }
        if let Some(gateway) = &libvirt.gateway {
            lines.push(format!("gateway: {gateway}"));
        }
        if !libvirt.dns_servers.is_empty() {
            lines.push(format!("dns_servers: {}", libvirt.dns_servers.join(",")));
        }
        lines.push(format!(
            "expected_virsh_probe: virsh -c {} pool-list --all",
            libvirt.connection_uri
        ));
        lines.push(format!(
            "expected_overlay_create: virsh -c {} vol-create-as {} <domain>.qcow2 {} --format qcow2 --backing-vol {} --backing-vol-format qcow2",
            libvirt.connection_uri, libvirt.pool, libvirt.disk_size, libvirt.base_image
        ));
        lines.push(format!(
            "expected_virt_install: virt-install --connect {} --name <domain> ...",
            libvirt.connection_uri
        ));
    }

    lines.join("\n")
}

pub fn generate(args: &VerifyGenerateArgs) -> Result<VerificationCommandResult, CoreError> {
    let spec_text = std::fs::read_to_string(&args.spec).map_err(|err| {
        CoreError::new(
            FailureClass::Validation,
            format!("failed to read spec {}: {err}", args.spec.display()),
        )
    })?;
    let accepted = load_accepted_corpus(&args.accepted_dir)?;
    let coverage = build_coverage_report(&spec_text, &accepted)?;
    let candidates = generate_candidates_from_spec(&spec_text, &accepted)?;

    if let Some(output_dir) = &args.output_dir {
        std::fs::create_dir_all(output_dir).map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!(
                    "failed to create output dir {}: {err}",
                    output_dir.display()
                ),
            )
        })?;
    }

    let mut rendered = String::new();
    let mut exit_code = 0;
    for candidate in &candidates {
        let yaml = render_candidate_yaml(candidate)?;
        rendered.push_str(&yaml);
        if !rendered.ends_with('\n') {
            rendered.push('\n');
        }
        if let Some(output_dir) = &args.output_dir {
            let path = output_dir.join(format!("{}.yaml", candidate.candidate_id));
            std::fs::write(&path, &yaml).map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!(
                        "failed to write generated candidate {}: {err}",
                        path.display()
                    ),
                )
            })?;
        }
        if candidate.review_status
            == crate::core::verification_model::VerificationCandidateReviewStatus::Rejected
        {
            exit_code = 1;
        }
    }
    rendered.push_str(&format_verification_coverage_report(&coverage));

    Ok(VerificationCommandResult::Generate {
        human_report: rendered,
        exit_code,
    })
}

pub fn execute_scenario(
    scenario: &VerificationScenarioDefinition,
    mode: VerificationRunMode,
    run_id: &str,
    context: &VerificationExecutionContext<'_>,
    verbose: bool,
    pause_before_teardown: bool,
) -> Result<VerificationRunView, CoreError> {
    let started_at = now_rfc3339()?;
    std::fs::create_dir_all(context.workspace).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create verification workspace {}: {err}",
                context.workspace.display()
            ),
        )
    })?;
    std::fs::create_dir_all(context.artifacts_root).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create artifacts root {}: {err}",
                context.artifacts_root.display()
            ),
        )
    })?;
    let artifact_workspace = context.artifacts_root.join(run_id);
    std::fs::create_dir_all(&artifact_workspace).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create artifact workspace {}: {err}",
                artifact_workspace.display()
            ),
        )
    })?;

    let guest = context
        .libvirt
        .create_guest(scenario, context.workspace)
        .map_err(|err| augment_pre_guest_error(err, context.workspace, &artifact_workspace))?;
    write_env_backed_debug_artifacts(&guest, &artifact_workspace)?;
    if verbose {
        if let Some(assigned_ip) = &guest.assigned_ip {
            eprintln!("assigned_ip: {assigned_ip}");
        }
        if let Some(network_config) = &guest.rendered_network_config {
            eprintln!("static_network_config:\n{network_config}");
        }
        if guest.env_backed {
            eprintln!(
                "debug_artifacts: {}/artifacts",
                artifact_workspace.display()
            );
        }
    }
    let runtime_bindings = prepare_runtime_bindings(
        scenario,
        run_id,
        context.workspace,
        &guest,
        context.guest_boundary,
    )
    .map_err(|err| augment_early_env_backed_error(err, &artifact_workspace, &guest))?;
    let plan = build_execution_plan(scenario, run_id.to_string(), mode, runtime_bindings.as_ref())?;
    let mut step_results = Vec::new();
    let scenario_timeout = parse_timeout_literal(&scenario.effective_timeouts()?.scenario_timeout)?;
    let scenario_started = Instant::now();

    for step in &plan.step_sequence {
        let mut step_result = match step.step_type {
            VerificationStepType::Boot => VerificationStepResult {
                step_id: step.step_id.clone(),
                status: VerificationStepStatus::Passed,
                details: Some(format!("booted {}", guest.guest_name)),
                command: None,
                exit_code: Some(0),
                stdout: Some(format!("booted {}", guest.guest_name)),
                stderr: None,
                duration_ms: Some(0),
            },
            VerificationStepType::WaitReady => {
                let started = Instant::now();
                match context
                    .guest_boundary
                    .wait_ready(&guest, &step.effective_timeout)
                {
                    Ok(output) => VerificationStepResult {
                        step_id: step.step_id.clone(),
                        status: VerificationStepStatus::Passed,
                        details: Some(output.stdout.clone()),
                        command: Some("wait_ready".to_string()),
                        exit_code: Some(0),
                        stdout: Some(output.stdout),
                        stderr: Some(output.stderr),
                        duration_ms: Some(started.elapsed().as_millis() as u64),
                    },
                    Err(err) if err.class == FailureClass::Transient => VerificationStepResult {
                        step_id: step.step_id.clone(),
                        status: VerificationStepStatus::TimedOut,
                        details: Some(err.message),
                        command: Some("wait_ready".to_string()),
                        exit_code: None,
                        stdout: None,
                        stderr: None,
                        duration_ms: Some(started.elapsed().as_millis() as u64),
                    },
                    Err(err) => return Err(err),
                }
            }
            VerificationStepType::CoreopsAction
            | VerificationStepType::GuestCommand
            | VerificationStepType::MutateState
            | VerificationStepType::Reboot => {
                let command = step.command_or_action.as_deref().unwrap_or("noop");
                let started = Instant::now();
                match context.guest_boundary.run_command(
                    &guest,
                    command,
                    Some(step.effective_timeout.as_str()),
                ) {
                    Ok(output) => {
                        let status = if output.status_code == 0 {
                            VerificationStepStatus::Passed
                        } else {
                            VerificationStepStatus::Failed
                        };
                        VerificationStepResult {
                            step_id: step.step_id.clone(),
                            status,
                            details: Some(output.stdout.clone()),
                            command: Some(command.to_string()),
                            exit_code: Some(output.status_code),
                            stdout: Some(output.stdout),
                            stderr: Some(output.stderr),
                            duration_ms: Some(started.elapsed().as_millis() as u64),
                        }
                    }
                    Err(err) if err.class == FailureClass::Transient => VerificationStepResult {
                        step_id: step.step_id.clone(),
                        status: VerificationStepStatus::TimedOut,
                        details: Some(err.message),
                        command: Some(command.to_string()),
                        exit_code: None,
                        stdout: None,
                        stderr: None,
                        duration_ms: Some(started.elapsed().as_millis() as u64),
                    },
                    Err(err) => return Err(err),
                }
            }
        };
        if scenario_started.elapsed() > scenario_timeout {
            step_result.status = VerificationStepStatus::TimedOut;
            step_result.details = Some(format!(
                "scenario timeout exceeded after {} ms (limit {})",
                scenario_started.elapsed().as_millis(),
                scenario.effective_timeouts()?.scenario_timeout
            ));
        }
        step_results.push(step_result);
        if matches!(step_results.last().map(|step| step.status), Some(VerificationStepStatus::TimedOut))
        {
            break;
        }
    }

    let mut assertion_results = evaluate_assertions(&scenario.assertions, &step_results)?;
    for result in &mut assertion_results {
        result.observed_value = result
            .observed_value
            .as_deref()
            .map(strip_ansi_escapes);
    }

    let overall_outcome = classify_scenario_outcome(&step_results, &assertion_results);
    let mut diagnostic_warnings = Vec::new();
    if guest.env_backed && overall_outcome != VerificationRunOutcome::Passed {
        if let Err(err) = collect_env_backed_failure_diagnostics(
            context.guest_boundary,
            &guest,
            &artifact_workspace,
            &step_results,
        ) {
            diagnostic_warnings.push(format!(
                "failed to collect env-backed guest diagnostics: {err}"
            ));
        }
    }
    let failure_summary = match overall_outcome {
        VerificationRunOutcome::Passed => None,
        VerificationRunOutcome::AssertionFailure => {
            Some("one or more verification assertions failed".to_string())
        }
        VerificationRunOutcome::InfrastructureFailure => {
            Some("guest provisioning or command execution failed".to_string())
        }
        VerificationRunOutcome::Timeout => Some("verification timed out".to_string()),
        VerificationRunOutcome::HarnessError => Some("verification harness error".to_string()),
    };
    let bundle_workspace = artifact_workspace.join("artifacts");
    let enrichment = write_diagnostic_artifacts(
        &bundle_workspace,
        scenario,
        overall_outcome,
        failure_summary.as_deref(),
        &step_results,
        &assertion_results,
    )?;
    write_execution_artifacts(&artifact_workspace, &step_results, &assertion_results)?;
    let mut artifacts = context
        .artifact_boundary
        .collect_artifacts(scenario, &artifact_workspace)?;
    artifacts.warnings.extend(diagnostic_warnings);
    artifacts.bundle.environment_retained = plan.retain_environment;
    context
        .artifact_boundary
        .write_bundle_manifest(&artifacts.bundle)?;

    if pause_before_teardown && !plan.retain_environment {
        wait_for_teardown_ack(run_id, &artifact_workspace)?;
    }
    if !plan.retain_environment {
        context.libvirt.destroy_guest(&guest)?;
    }
    let completed_at = now_rfc3339()?;

    let _run = VerificationRun {
        run_id: run_id.to_string(),
        mode,
        revision_selection_basis: VerificationRevisionSelectionBasis::SingleScenario,
        revision_under_test: scenario.fixtures.revision_under_test.clone(),
        controller_version: env!("CARGO_PKG_VERSION").to_string(),
        scenario_refs: vec![scenario.scenario_id.clone()],
        workspace_path: context.workspace.display().to_string(),
        started_at: started_at.clone(),
        completed_at: completed_at.clone(),
        overall_outcome,
        artifact_bundle: artifacts.bundle.clone(),
    };
    let _scenario_outcome = VerificationScenarioOutcome {
        scenario_id: scenario.scenario_id.clone(),
        revision_under_test: scenario.fixtures.revision_under_test.clone(),
        outcome: overall_outcome,
        step_results: step_results.clone(),
        assertion_results: assertion_results.clone(),
        failure_summary: failure_summary.clone(),
    };

    Ok(VerificationRunView {
        view_kind: "verification_run".to_string(),
        run_id: run_id.to_string(),
        mode,
        controller_version: env!("CARGO_PKG_VERSION").to_string(),
        revision_selection_basis: VerificationRevisionSelectionBasis::SingleScenario,
        revision_under_test: scenario.fixtures.revision_under_test.clone(),
        started_at,
        completed_at,
        scenario_id: scenario.scenario_id.clone(),
        title: scenario.title.clone(),
        overall_outcome,
        artifact_bundle: artifacts.bundle,
        environment_retained: plan.retain_environment,
        step_results,
        assertion_results,
        warnings: artifacts.warnings,
        failure_summary,
        regression_summary: enrichment.regression_summary,
        promotion_status: enrichment.promotion_status,
    })
}

fn now_rfc3339() -> Result<String, CoreError> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|err| CoreError::new(FailureClass::Verify, format!("failed to format timestamp: {err}")))
}

fn validate_run_args(args: &VerifyRunArgs) -> Result<(), CoreError> {
    if args.pause_before_teardown && !args.debug {
        return Err(CoreError::new(
            FailureClass::Validation,
            "--pause-before-teardown requires --debug",
        ));
    }
    if args.pause_before_teardown && args.ci {
        return Err(CoreError::new(
            FailureClass::Validation,
            "--pause-before-teardown cannot be used with --ci",
        ));
    }
    if args.pause_before_teardown && args.accepted_dir.is_some() {
        return Err(CoreError::new(
            FailureClass::Validation,
            "--pause-before-teardown is supported only for single-scenario runs",
        ));
    }
    Ok(())
}

fn wait_for_teardown_ack(run_id: &str, artifact_workspace: &Path) -> Result<(), CoreError> {
    eprintln!(
        "Debug pause before teardown for {run_id}. Artifacts are in {}. Press Enter to continue teardown.",
        artifact_workspace.join("artifacts").display()
    );
    io::stderr().flush().map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to flush debug pause prompt: {err}"),
        )
    })?;
    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to read debug pause acknowledgement: {err}"),
        )
    })?;
    Ok(())
}

fn format_verification_suite_json(view: &VerificationSuiteJsonView<'_>) -> String {
    serde_json::json!({
        "view_kind": "verification_run",
        "run_id": view.run_id,
        "mode": view.mode,
        "controller_version": env!("CARGO_PKG_VERSION"),
        "revision_selection_basis": VerificationRevisionSelectionBasis::AcceptedCorpus,
        "revision_under_test": view.revision_under_test,
        "overall_outcome": view.overall_outcome,
        "started_at": view.started_at,
        "completed_at": view.completed_at,
        "scenario_outcomes": view.scenario_outcomes.iter().map(|outcome| serde_json::json!({
            "scenario_id": &outcome.scenario_id,
            "revision_under_test": &outcome.revision_under_test,
            "outcome": outcome.outcome,
            "failure_summary": &outcome.failure_summary,
            "assertion_results": outcome.assertion_results.iter().map(|result| serde_json::json!({
                "assertion_id": &result.assertion_id,
                "status": result.status,
                "observed_value": &result.observed_value,
                "evidence_refs": &result.evidence_refs,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "artifacts": {
            "bundle_path": view.bundle_path,
            "environment_retained": view.environment_retained,
        }
    })
    .to_string()
}

fn write_suite_bundle_index(
    bundle_path: &str,
    views: &[VerificationRunView],
) -> Result<(), CoreError> {
    let suite_root = Path::new(bundle_path);
    std::fs::create_dir_all(suite_root).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to create suite bundle root {}: {err}", suite_root.display()),
        )
    })?;
    let payload = serde_json::json!({
        "scenario_bundles": views.iter().map(|view| serde_json::json!({
            "scenario_id": &view.scenario_id,
            "revision_under_test": &view.revision_under_test,
            "bundle_path": &view.artifact_bundle.bundle_path,
            "outcome": view.overall_outcome,
        })).collect::<Vec<_>>(),
    });
    std::fs::write(
        suite_root.join("scenario-bundles.json"),
        serde_json::to_string_pretty(&payload).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to serialize suite bundle index: {err}"),
            )
        })?,
    )
    .map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to write suite bundle index: {err}"),
        )
    })
}

fn summarize_suite_revision_under_test(
    scenario_outcomes: &[VerificationScenarioOutcome],
) -> String {
    let revisions = scenario_outcomes
        .iter()
        .map(|outcome| outcome.revision_under_test.clone())
        .collect::<BTreeSet<_>>();
    if revisions.len() == 1 {
        revisions.into_iter().next().unwrap_or_else(|| "accepted-corpus".to_string())
    } else {
        "accepted-corpus".to_string()
    }
}

fn strip_ansi_escapes(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn collect_env_backed_failure_diagnostics(
    guest_boundary: &dyn VerificationGuestBoundary,
    guest: &crate::core::verification_model::LibvirtGuestHandle,
    artifact_workspace: &Path,
    step_results: &[VerificationStepResult],
) -> Result<(), CoreError> {
    let bundle_root = artifact_workspace.join("artifacts");
    std::fs::create_dir_all(&bundle_root).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create failure diagnostic artifact root {}: {err}",
                bundle_root.display()
            ),
        )
    })?;

    let diagnostics = [
        (
            "guest-journal.txt",
            "sudo journalctl -b --no-pager",
            "guest boot journal",
        ),
        (
            "systemctl-failed.txt",
            "sudo systemctl --failed --no-pager --full",
            "failed systemd units",
        ),
        (
            "coreops-status.json",
            "sudo cat /var/lib/core-ops/status.json",
            "core-ops state file",
        ),
        (
            "quadlet-dir-list.txt",
            "sudo ls -al /etc/containers/systemd",
            "quadlet directory listing",
        ),
        (
            "systemd-dir-list.txt",
            "sudo ls -al /etc/systemd/system",
            "systemd unit directory listing",
        ),
    ];

    for (file_name, command, description) in diagnostics {
        let output = guest_boundary.run_command(guest, command, Some("60s"))?;
        write_guest_diagnostic_artifact(&bundle_root.join(file_name), command, description, &output)?;
    }

    for unit in infer_failure_unit_hints(step_results) {
        let sanitized = sanitize_artifact_name(&unit);
        let status_command =
            format!("sudo systemctl status --no-pager --full {}", shell_escape(&unit));
        let status_output = guest_boundary.run_command(guest, &status_command, Some("60s"))?;
        write_guest_diagnostic_artifact(
            &bundle_root.join(format!("unit-status-{sanitized}.txt")),
            &status_command,
            &format!("systemd status for {unit}"),
            &status_output,
        )?;

        let journal_command = format!(
            "sudo journalctl -u {} -b --no-pager",
            shell_escape(&unit)
        );
        let journal_output = guest_boundary.run_command(guest, &journal_command, Some("60s"))?;
        write_guest_diagnostic_artifact(
            &bundle_root.join(format!("unit-journal-{sanitized}.txt")),
            &journal_command,
            &format!("boot journal for {unit}"),
            &journal_output,
        )?;
    }

    Ok(())
}

fn write_guest_diagnostic_artifact(
    path: &Path,
    command: &str,
    description: &str,
    output: &crate::core::verification_model::GuestCommandOutput,
) -> Result<(), CoreError> {
    let mut payload = format!(
        "description: {description}\ncommand: {command}\nexit_code: {}\n",
        output.status_code
    );
    if !output.stdout.trim().is_empty() {
        payload.push_str("stdout:\n");
        payload.push_str(&output.stdout);
        if !output.stdout.ends_with('\n') {
            payload.push('\n');
        }
    }
    if !output.stderr.trim().is_empty() {
        payload.push_str("stderr:\n");
        payload.push_str(&output.stderr);
        if !output.stderr.ends_with('\n') {
            payload.push('\n');
        }
    }
    std::fs::write(path, payload).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to write guest diagnostic artifact {}: {err}", path.display()),
        )
    })
}

fn infer_failure_unit_hints(step_results: &[VerificationStepResult]) -> Vec<String> {
    let mut units = std::collections::BTreeSet::new();
    for step in step_results {
        for text in [&step.stdout, &step.stderr] {
            let Some(text) = text else {
                continue;
            };
            for line in text.lines() {
                if let Some(unit) = line
                    .split("systemctl status ")
                    .nth(1)
                    .and_then(|rest| rest.split_whitespace().next())
                {
                    if is_systemd_unit_name(unit) {
                        units.insert(unit.to_string());
                    }
                }
                if let Some(unit) = line
                    .split("journalctl -u ")
                    .nth(1)
                    .and_then(|rest| rest.split_whitespace().next())
                {
                    if is_systemd_unit_name(unit) {
                        units.insert(unit.to_string());
                    }
                }
            }
        }
    }
    units.into_iter().collect()
}

fn is_systemd_unit_name(value: &str) -> bool {
    value.ends_with(".service")
        || value.ends_with(".socket")
        || value.ends_with(".mount")
        || value.ends_with(".automount")
}

fn sanitize_artifact_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn write_execution_artifacts(
    artifact_workspace: &Path,
    step_results: &[VerificationStepResult],
    assertion_results: &[VerificationAssertionResult],
) -> Result<(), CoreError> {
    let bundle_root = artifact_workspace.join("artifacts");
    std::fs::create_dir_all(&bundle_root).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create execution artifact root {}: {err}",
                bundle_root.display()
            ),
        )
    })?;

    let mut harness_log = String::new();
    let mut coreops_output = String::new();
    for step in step_results {
        harness_log.push_str(&format!(
            "step={} status={:?} exit_code={}\n",
            step.step_id,
            step.status,
            step.exit_code.unwrap_or(-1)
        ));
        if let Some(command) = &step.command {
            harness_log.push_str(&format!("command: {command}\n"));
        }
        if let Some(stdout) = &step.stdout {
            if !stdout.trim().is_empty() {
                harness_log.push_str("stdout:\n");
                harness_log.push_str(stdout);
                if !stdout.ends_with('\n') {
                    harness_log.push('\n');
                }
            }
        }
        if let Some(stderr) = &step.stderr {
            if !stderr.trim().is_empty() {
                harness_log.push_str("stderr:\n");
                harness_log.push_str(stderr);
                if !stderr.ends_with('\n') {
                    harness_log.push('\n');
                }
            }
        }
        harness_log.push('\n');

        if let Some(command) = &step.command {
            if command.contains("core-ops") {
                coreops_output.push_str(&format!("step={}\ncommand: {command}\n", step.step_id));
                if let Some(stdout) = &step.stdout {
                    coreops_output.push_str("stdout:\n");
                    coreops_output.push_str(stdout);
                    if !stdout.ends_with('\n') {
                        coreops_output.push('\n');
                    }
                }
                if let Some(stderr) = &step.stderr {
                    if !stderr.trim().is_empty() {
                        coreops_output.push_str("stderr:\n");
                        coreops_output.push_str(stderr);
                        if !stderr.ends_with('\n') {
                            coreops_output.push('\n');
                        }
                    }
                }
                coreops_output.push('\n');
            }
        }
    }

    let assertion_payload = serde_json::to_string_pretty(assertion_results).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to serialize assertion results: {err}"),
        )
    })?;

    std::fs::write(bundle_root.join("harness-log.txt"), harness_log).map_err(|err| {
        CoreError::new(FailureClass::Apply, format!("failed to write harness-log.txt: {err}"))
    })?;
    std::fs::write(bundle_root.join("coreops-output.txt"), coreops_output).map_err(|err| {
        CoreError::new(FailureClass::Apply, format!("failed to write coreops-output.txt: {err}"))
    })?;
    std::fs::write(bundle_root.join("assertion-results.txt"), assertion_payload).map_err(
        |err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to write assertion-results.txt: {err}"),
            )
        },
    )?;

    Ok(())
}

fn write_env_backed_debug_artifacts(
    guest: &crate::core::verification_model::LibvirtGuestHandle,
    artifact_workspace: &Path,
) -> Result<(), CoreError> {
    let bundle_root = artifact_workspace.join("artifacts");
    std::fs::create_dir_all(&bundle_root).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create debug artifact bundle root {}: {err}",
                bundle_root.display()
            ),
        )
    })?;

    if let Some(network_config) = &guest.rendered_network_config {
        let path = bundle_root.join("static-network-config.txt");
        std::fs::write(&path, network_config).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to write {}: {err}", path.display()),
            )
        })?;
    }
    if let Some(local_butane_path) = &guest.local_butane_path {
        copy_debug_file(local_butane_path, &bundle_root.join("rendered-ignition.bu"))?;
    }
    if let Some(local_ignition_path) = &guest.local_ignition_path {
        copy_debug_file(local_ignition_path, &bundle_root.join("rendered-ignition.ign"))?;
    }

    if guest.env_backed {
        let console_path = bundle_root.join("console-log.txt");
        let console_log = fetch_serial_console_log(guest)?;
        std::fs::write(&console_path, console_log).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to write {}: {err}", console_path.display()),
            )
        })?;

        let qemu_launch_path = bundle_root.join("qemu-launch-log.txt");
        let qemu_launch_log = fetch_qemu_launch_log(guest)?;
        std::fs::write(&qemu_launch_path, qemu_launch_log).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to write {}: {err}", qemu_launch_path.display()),
            )
        })?;
    }

    Ok(())
}

fn copy_debug_file(source: &str, dest: &Path) -> Result<(), CoreError> {
    std::fs::copy(source, dest).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to copy debug file {source} to {}: {err}", dest.display()),
        )
    })?;
    Ok(())
}

fn copy_workspace_debug_renders(
    workspace: &Path,
    artifact_workspace: &Path,
) -> Result<(), CoreError> {
    let bundle_root = artifact_workspace.join("artifacts");
    std::fs::create_dir_all(&bundle_root).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create pre-guest debug artifact root {}: {err}",
                bundle_root.display()
            ),
        )
    })?;

    for entry in std::fs::read_dir(workspace).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to read workspace {}: {err}", workspace.display()),
        )
    })? {
        let entry = entry.map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to read workspace entry: {err}"),
            )
        })?;
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let dest = match ext {
            "bu" => bundle_root.join("rendered-ignition.bu"),
            "ign" => bundle_root.join("rendered-ignition.ign"),
            _ => continue,
        };
        if name == "rendered-ignition.bu" || name == "rendered-ignition.ign" {
            continue;
        }
        std::fs::copy(&path, &dest).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to copy {} to {}: {err}", path.display(), dest.display()),
            )
        })?;
    }

    Ok(())
}

fn fetch_serial_console_log(
    guest: &crate::core::verification_model::LibvirtGuestHandle,
) -> Result<String, CoreError> {
    let log_path = guest
        .serial_log_path
        .as_deref()
        .unwrap_or("<missing serial log path>");
    fetch_hypervisor_file(guest, log_path, "guest serial console log")
}

fn fetch_qemu_launch_log(
    guest: &crate::core::verification_model::LibvirtGuestHandle,
) -> Result<String, CoreError> {
    let log_path = guest
        .qemu_launch_log_path
        .as_deref()
        .unwrap_or("<missing qemu launch log path>");
    fetch_hypervisor_file(guest, log_path, "qemu launch log")
}

fn fetch_hypervisor_file(
    guest: &crate::core::verification_model::LibvirtGuestHandle,
    log_path: &str,
    description: &str,
) -> Result<String, CoreError> {
    let output = if let Some(vm_host) = &guest.vm_host {
        let ssh_user = guest.ssh_user.as_deref().unwrap_or("core");
        let mut command = Command::new("ssh");
        command
            .arg(format!("{ssh_user}@{vm_host}"))
            .arg(format!("sudo cat {}", shell_escape(log_path)));
        command.output().map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to fetch {description} over ssh: {err}"),
            )
        })?
    } else {
        let mut command = Command::new("sudo");
        command.arg("cat").arg(log_path);
        command.output().map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to read local {description}: {err}"),
            )
        })?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(format!(
            "{description} unavailable at {log_path}\n{}",
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn augment_early_env_backed_error(
    err: CoreError,
    artifact_workspace: &Path,
    guest: &crate::core::verification_model::LibvirtGuestHandle,
) -> CoreError {
    if !guest.env_backed {
        return err;
    }

    let artifacts_path = artifact_workspace.join("artifacts");
    let console_path = artifacts_path.join("console-log.txt");
    let network_path = artifacts_path.join("static-network-config.txt");
    let butane_path = artifacts_path.join("rendered-ignition.bu");
    let ignition_path = artifacts_path.join("rendered-ignition.ign");
    CoreError::new(
        err.class,
        format!(
            "{}\nDebug artifacts: {}\nConsole log: {}\nStatic network config: {}\nRendered Butane: {}\nRendered Ignition: {}",
            err.message,
            artifacts_path.display(),
            console_path.display(),
            network_path.display(),
            butane_path.display(),
            ignition_path.display()
        ),
    )
}

fn augment_pre_guest_error(
    err: CoreError,
    workspace: &Path,
    artifact_workspace: &Path,
) -> CoreError {
    let _ = copy_workspace_debug_renders(workspace, artifact_workspace);
    let artifacts_path = artifact_workspace.join("artifacts");
    let butane_path = artifacts_path.join("rendered-ignition.bu");
    let ignition_path = artifacts_path.join("rendered-ignition.ign");
    CoreError::new(
        err.class,
        format!(
            "{}\nDebug artifacts: {}\nRendered Butane: {}\nRendered Ignition: {}",
            err.message,
            artifacts_path.display(),
            butane_path.display(),
            ignition_path.display()
        ),
    )
}

fn prepare_runtime_bindings(
    scenario: &VerificationScenarioDefinition,
    run_id: &str,
    workspace: &Path,
    guest: &crate::core::verification_model::LibvirtGuestHandle,
    guest_boundary: &dyn VerificationGuestBoundary,
) -> Result<Option<VerificationRuntimeBindings>, CoreError> {
    if !guest.env_backed {
        return Ok(None);
    }
    let timeouts = scenario.effective_timeouts()?;
    guest_boundary.wait_ready(guest, &timeouts.readiness_timeout)?;

    let local_runtime = workspace.join("runtime");
    let local_repo = materialize_repo_fixture(scenario, &local_runtime)?;
    let core_ops_binary = resolve_core_ops_binary()?;

    guest_boundary.run_command(
        guest,
        &format!(
            "mkdir -p /var/tmp/core-ops-verify/{run_id}/bin /var/tmp/core-ops-verify/{run_id}/repo /var/tmp/core-ops-verify/{run_id}/out"
        ),
        None,
    )?;

    let remote_root = format!("/var/tmp/core-ops-verify/{run_id}");
    let remote_binary = format!("{remote_root}/bin/core-ops");
    let remote_repo_parent = format!("{remote_root}/repo");
    let remote_repo_checkout = format!("{remote_repo_parent}/repo");
    let remote_quadlet_dir = "/etc/containers/systemd".to_string();
    let remote_systemd_dir = "/etc/systemd/system".to_string();
    let remote_state = format!("{remote_root}/out/status.json");

    guest_boundary.copy_to_guest(guest, &core_ops_binary, &remote_binary, false, true)?;
    guest_boundary.copy_to_guest(guest, &local_repo, &remote_repo_parent, true, false)?;

    Ok(Some(VerificationRuntimeBindings {
        repo_path: format!("file://{remote_repo_checkout}"),
        core_ops_binary: remote_binary,
        quadlet_dir: remote_quadlet_dir,
        systemd_unit_dir: remote_systemd_dir,
        state_file: remote_state,
    }))
}

fn materialize_repo_fixture(
    scenario: &VerificationScenarioDefinition,
    local_runtime: &Path,
) -> Result<std::path::PathBuf, CoreError> {
    let materialized = local_runtime.join("repo");
    std::fs::create_dir_all(local_runtime).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to create local verification runtime workspace: {err}"),
        )
    })?;

    if let Some(evolution) = &scenario.fixtures.repository_evolution {
        if let Some(history_fixture) = &evolution.history_fixture {
            return materialize_history_repo(history_fixture, &evolution.revisions, &materialized);
        }
    }

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(&scenario.fixtures.repo_fixture);
    copy_tree(&source, &materialized)?;
    Ok(materialized)
}

fn materialize_history_repo(
    history_fixture: &str,
    revisions: &[String],
    target: &Path,
) -> Result<std::path::PathBuf, CoreError> {
    let history_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(history_fixture);
    std::fs::create_dir_all(target).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to create materialized repo target {}: {err}", target.display()),
        )
    })?;
    let mut init = Command::new("git");
    init.arg("init").arg(target);
    run_local_command(&mut init, "git init materialized verification repo")?;

    for revision in revisions {
        let revision_dir = history_root.join(revision);
        if !revision_dir.exists() {
            return Err(CoreError::new(
                FailureClass::Validation,
                format!(
                    "repository evolution fixture {} is missing revision {}",
                    history_root.display(),
                    revision
                ),
            ));
        }
        clear_directory_contents(target)?;
        copy_tree(&revision_dir, target)?;
        let mut add = Command::new("git");
        add.arg("-C").arg(target).arg("add").arg(".");
        run_local_command(&mut add, "git add materialized verification repo")?;

        let mut commit = Command::new("git");
        commit
            .arg("-C")
            .arg(target)
            .arg("commit")
            .arg("-m")
            .arg(format!("fixture {revision}"))
            .env("GIT_AUTHOR_NAME", "fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
            .env("GIT_COMMITTER_NAME", "fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.com");
        run_local_command(&mut commit, "git commit materialized verification repo")?;

        let mut tag = Command::new("git");
        tag.arg("-C").arg(target).arg("tag").arg("-f").arg(revision);
        run_local_command(&mut tag, "git tag materialized verification repo")?;
    }

    Ok(target.to_path_buf())
}

fn clear_directory_contents(path: &Path) -> Result<(), CoreError> {
    for entry in std::fs::read_dir(path).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to read directory {}: {err}", path.display()),
        )
    })? {
        let entry = entry.map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to read directory entry: {err}"),
            )
        })?;
        let entry_path = entry.path();
        if entry.file_name() == ".git" {
            continue;
        }
        if entry_path.is_dir() {
            std::fs::remove_dir_all(&entry_path).map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format!("failed to remove {}: {err}", entry_path.display()),
                )
            })?;
        } else {
            std::fs::remove_file(&entry_path).map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format!("failed to remove {}: {err}", entry_path.display()),
                )
            })?;
        }
    }
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), CoreError> {
    if !source.exists() {
        return Err(CoreError::new(
            FailureClass::Validation,
            format!("verification repo fixture {} does not exist", source.display()),
        ));
    }
    if source.is_file() {
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format!("failed to create {}: {err}", parent.display()),
                )
            })?;
        }
        std::fs::copy(source, target).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to copy {} to {}: {err}", source.display(), target.display()),
            )
        })?;
        return Ok(());
    }

    std::fs::create_dir_all(target).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to create {}: {err}", target.display()),
        )
    })?;
    for entry in std::fs::read_dir(source).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to read {}: {err}", source.display()),
        )
    })? {
        let entry = entry.map_err(|err| {
            CoreError::new(FailureClass::Apply, format!("failed to read entry: {err}"))
        })?;
        let entry_path = entry.path();
        let dest_path = target.join(entry.file_name());
        if entry_path.is_dir() {
            copy_tree(&entry_path, &dest_path)?;
        } else {
            std::fs::copy(&entry_path, &dest_path).map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format!(
                        "failed to copy {} to {}: {err}",
                        entry_path.display(),
                        dest_path.display()
                    ),
                )
            })?;
        }
    }
    Ok(())
}

fn resolve_core_ops_binary() -> Result<std::path::PathBuf, CoreError> {
    if let Ok(path) = std::env::var("CORE_OPS_VERIFY_CORE_OPS_BIN") {
        let path = std::path::PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let candidate = parent.join("core-ops");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Err(CoreError::new(
        FailureClass::Validation,
        "unable to locate core-ops binary for VM-backed verification; build it first or set CORE_OPS_VERIFY_CORE_OPS_BIN",
    ))
}

fn run_local_command(command: &mut Command, context: &str) -> Result<(), CoreError> {
    let output = command.output().map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to launch {context}: {err}"),
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() && !stdout.is_empty() {
            format!("stderr: {stderr}; stdout: {stdout}")
        } else if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            format!("stdout: {stdout}")
        } else {
            "command exited unsuccessfully with no output".to_string()
        };
        return Err(CoreError::new(
            FailureClass::Apply,
            format!("{context} failed: {detail}"),
        ));
    }
    Ok(())
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn next_run_id(scenario_id: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    format!("run-{nanos}-{scenario_id}")
}
