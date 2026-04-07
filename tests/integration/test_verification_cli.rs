use std::io::Write;
use std::process::{Command, Stdio};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn cli_verification_run_in_local_mode_tears_environment_down() {
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--scenario")
        .arg(fixture_path(
            "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
        ))
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .arg("--synthetic")
        .output()
        .expect("run cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Verification run verify-idempotent-frontend [local]"));
    assert!(stdout.contains("Env:     torn down"));
}

#[test]
fn cli_verification_run_in_debug_mode_retains_environment() {
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--scenario")
        .arg(fixture_path(
            "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
        ))
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .arg("--debug")
        .arg("--synthetic")
        .output()
        .expect("run cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Verification run verify-idempotent-frontend [debug]"));
    assert!(stdout.contains("Env:     retained"));
}

#[test]
fn cli_verification_run_can_pause_before_teardown_in_debug_mode() {
    let scenario_dir = tempfile::tempdir().expect("scenario dir");
    let scenario_yaml = std::fs::read_to_string(fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("read fixture");
    let scenario_path = scenario_dir.path().join("pause-before-teardown.yaml");
    std::fs::write(
        &scenario_path,
        format!(
            "{scenario_yaml}\npolicy_overrides:\n  artifact_policy:\n    retain_environment_in_debug: false\n    export_format: directory\n"
        ),
    )
    .expect("write scenario");

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let mut child = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--scenario")
        .arg(&scenario_path)
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .arg("--debug")
        .arg("--pause-before-teardown")
        .arg("--synthetic")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cli");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"\n")
        .expect("ack pause");
    let output = child.wait_with_output().expect("wait for cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Env:     torn down"));
    assert!(stderr.contains("Press Enter to continue teardown"));
}

#[test]
fn cli_verification_run_rejects_pause_before_teardown_without_debug() {
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--scenario")
        .arg(fixture_path(
            "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
        ))
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .arg("--pause-before-teardown")
        .arg("--synthetic")
        .output()
        .expect("run cli");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--pause-before-teardown requires --debug"));
}

#[test]
fn cli_verification_run_in_json_mode_emits_machine_readable_contract() {
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--scenario")
        .arg(fixture_path(
            "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
        ))
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .arg("--synthetic")
        .arg("--json")
        .output()
        .expect("run cli");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert_eq!(parsed["view_kind"], "verification_run");
    assert_eq!(parsed["overall_outcome"], "passed");
    assert_eq!(parsed["revision_selection_basis"], "single_scenario");
    assert_eq!(parsed["scenario_outcomes"][0]["scenario_id"], "verify-idempotent-frontend");
    assert_eq!(parsed["scenario_outcomes"][0]["revision_under_test"], "demo-uat-v2");
    assert!(parsed["started_at"].as_str().expect("started_at").ends_with('Z'));
    assert!(parsed["completed_at"].as_str().expect("completed_at").ends_with('Z'));
    assert!(!parsed["scenario_outcomes"][0]["assertion_results"][0]["observed_value"]
        .as_str()
        .expect("observed_value")
        .contains('\u{1b}'));
}

#[test]
fn cli_verification_run_in_ci_mode_uses_only_accepted_corpus_entries() {
    let corpus = tempfile::tempdir().expect("corpus");
    std::fs::copy(
        fixture_path("tests/fixtures/verification/scenarios/minimal-accepted.yaml"),
        corpus.path().join("minimal-accepted.yaml"),
    )
    .expect("copy accepted fixture");
    std::fs::copy(
        fixture_path("tests/fixtures/verification/scenarios/minimal-candidate.yaml"),
        corpus.path().join("minimal-candidate.yaml"),
    )
    .expect("copy candidate fixture");
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--accepted-dir")
        .arg(corpus.path())
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .arg("--synthetic")
        .arg("--ci")
        .arg("--json")
        .output()
        .expect("run ci corpus");

    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid suite json");
    assert_eq!(parsed["view_kind"], "verification_run");
    assert_eq!(parsed["mode"], "ci");
    assert_eq!(parsed["revision_selection_basis"], "accepted_corpus");
    assert_eq!(parsed["revision_under_test"], "demo-uat-v2");
    let outcomes = parsed["scenario_outcomes"]
        .as_array()
        .expect("scenario outcomes array");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0]["scenario_id"], "verify-idempotent-frontend");
    assert_eq!(outcomes[0]["revision_under_test"], "demo-uat-v2");
    let bundle_path = parsed["artifacts"]["bundle_path"]
        .as_str()
        .expect("bundle path");
    let index: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(std::path::Path::new(bundle_path).join("scenario-bundles.json"))
            .expect("read scenario bundles index"),
    )
    .expect("valid scenario bundles index");
    assert_eq!(
        index["scenario_bundles"][0]["revision_under_test"],
        "demo-uat-v2"
    );
}

#[test]
fn cli_verification_run_can_select_specific_accepted_scenario_ids() {
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--accepted-dir")
        .arg(fixture_path("tests/fixtures/verification/scenarios"))
        .arg("--scenario-id")
        .arg("verify-idempotent-frontend")
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .arg("--synthetic")
        .arg("--ci")
        .arg("--json")
        .output()
        .expect("run filtered ci corpus");

    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid filtered suite json");
    let outcomes = parsed["scenario_outcomes"]
        .as_array()
        .expect("scenario outcomes array");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0]["scenario_id"], "verify-idempotent-frontend");
    assert_eq!(outcomes[0]["revision_under_test"], "demo-uat-v2");
}

#[test]
fn cli_verification_suite_json_uses_actual_environment_retention_state() {
    let accepted_dir = tempfile::tempdir().expect("accepted dir");
    let scenario_yaml = std::fs::read_to_string(fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("read fixture");
    std::fs::write(
        accepted_dir.path().join("minimal-accepted.yaml"),
        format!(
            "{scenario_yaml}\npolicy_overrides:\n  artifact_policy:\n    retain_environment_in_debug: false\n    export_format: directory\n"
        ),
    )
    .expect("write scenario");

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--accepted-dir")
        .arg(accepted_dir.path())
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .arg("--debug")
        .arg("--synthetic")
        .arg("--json")
        .output()
        .expect("run debug corpus");

    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid suite json");
    assert_eq!(parsed["mode"], "debug");
    assert_eq!(parsed["artifacts"]["environment_retained"], false);
}

#[test]
fn cli_generate_emits_review_ready_candidate_yaml() {
    let temp = tempfile::tempdir().expect("tempdir");
    let spec_path = temp.path().join("feature.md");
    std::fs::write(
        &spec_path,
        r#"
# Feature Specification: Generated

## Verification Guidance

### Observable Behaviors

- Applying an upgraded revision preserves deterministic transition summaries

### Invariants

- Transition summaries stay stable

### Idempotency Expectations

- Reapplying the upgraded revision does not add new changes

### Failure Modes

- Upgrade failures remain diagnosable

### Upgrade Considerations

- Prior and target revision context remain visible

### Required Scenario Classes

- upgrade_transition
"#,
    )
    .expect("write spec");

    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("generate")
        .arg("--spec")
        .arg(&spec_path)
        .arg("--accepted-dir")
        .arg(fixture_path("tests/fixtures/verification/scenarios"))
        .output()
        .expect("run generate");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("review_status: needs_review"));
    assert!(stdout.contains("behavioral_claim:"));
    assert!(stdout.contains("Coverage"));
    assert!(stdout.contains("Missing:"));
}

#[test]
fn cli_generate_rejects_duplicate_candidate_with_non_zero_exit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let spec_path = temp.path().join("duplicate.md");
    std::fs::write(
        &spec_path,
        r#"
# Feature Specification: Duplicate

## Verification Guidance

### Observable Behaviors

- Reapplying the same revision produces no managed changes

### Invariants

- Idempotent applies remain stable

### Idempotency Expectations

- Reapplying the same revision remains stable

### Failure Modes

- Assertion failures remain diagnosable

### Upgrade Considerations

- Revision continuity remains visible

### Required Scenario Classes

- idempotency
"#,
    )
    .expect("write spec");

    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("generate")
        .arg("--spec")
        .arg(&spec_path)
        .arg("--accepted-dir")
        .arg(fixture_path("tests/fixtures/verification/scenarios"))
        .output()
        .expect("run generate");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("review_status: rejected"));
    assert!(stdout.contains("duplicate accepted coverage"));
    assert!(stdout.contains("Coverage"));
}

#[test]
fn cli_generate_rejects_missing_mandatory_verification_guidance() {
    let temp = tempfile::tempdir().expect("tempdir");
    let spec_path = temp.path().join("missing-guidance.md");
    std::fs::write(
        &spec_path,
        r#"
# Feature Specification: Missing Guidance

## Functional Requirements

- The system SHALL reapply the same revision without reporting managed changes.
"#,
    )
    .expect("write spec");

    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("generate")
        .arg("--spec")
        .arg(&spec_path)
        .arg("--accepted-dir")
        .arg(fixture_path("tests/fixtures/verification/scenarios"))
        .output()
        .expect("run generate");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("feature spec must include a Verification Guidance"));
}

#[test]
fn cli_verification_run_can_verify_command_surface_and_timing_assertions() {
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let scenario_path = workspace.path().join("command-surfaces.yaml");
    std::fs::write(
        &scenario_path,
        r#"scenario_id: verify-command-surfaces-cli
title: Command surface verification remains stable
description: Machine-readable and human-readable command surfaces remain stable.
scenario_classes:
  - explain_apply_consistency
source: accepted
behavioral_claim: Public command surfaces remain stable across machine-readable, human-readable, and agent flows.
rationale: Guards verification scenarios that exercise public command contracts.
environment:
  profile: single-blessed-vm
fixtures:
  repo_fixture: fixtures/repos/frontend
  revision_under_test: demo-uat-v2
steps:
  - step_id: boot
    step_type: boot
    target: guest
  - step_id: plan-json
    step_type: coreops_action
    target: guest
    action:
      action: plan
      repository_source: fixture
      revision: demo-uat-v2
      mode: json
      output_contract: machine-readable
  - step_id: apply-humane
    step_type: coreops_action
    target: guest
    action:
      action: apply
      repository_source: fixture
      revision: demo-uat-v2
      mode: humane
  - step_id: agent-run
    step_type: coreops_action
    target: guest
    action:
      action: agent
      repository_source: fixture
      revision: demo-uat-v2
assertions:
  - assertion_id: plan-json
    assertion_type: step_command_contains
    target: plan-json
    expected_state: --json
    failure_message: plan should use json mode
  - assertion_id: plan-json-output
    assertion_type: step_stdout_contains
    target: plan-json
    expected_state: '"command":"plan"'
    failure_message: plan should emit machine-readable output
  - assertion_id: humane-output
    assertion_type: step_stdout_contains
    target: apply-humane
    expected_state: "Outcome: converged"
    failure_message: apply should emit humane output
  - assertion_id: timing
    assertion_type: step_duration_within_ms
    target: plan-json
    expected_state: "1000"
    failure_message: plan should stay within timing guardrail
  - assertion_id: agent-command
    assertion_type: step_command_contains
    target: agent-run
    expected_state: "core-ops agent"
    failure_message: agent should render agent command
"#,
    )
    .expect("write scenario");

    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--scenario")
        .arg(&scenario_path)
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .arg("--synthetic")
        .arg("--json")
        .output()
        .expect("run cli");

    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(parsed["overall_outcome"], "passed");
    assert_eq!(
        parsed["scenario_outcomes"][0]["scenario_id"],
        "verify-command-surfaces-cli"
    );
}
