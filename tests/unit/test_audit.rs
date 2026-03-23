use core_ops::core::audit::{build_audit_event, build_audit_record, format_audit_event_json, format_audit_record};
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, ObservedState,
    QuadletType, RestartPolicy, Workload,
};
use core_ops::core::planner::plan;

fn desired_state() -> DesiredState {
    DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: vec![Workload {
            name: "alpha".to_string(),
            quadlet_type: QuadletType::Container,
            quadlet_contents: "[Container]".to_string(),
            systemd_unit_name: "alpha.container".to_string(),
            enabled_state: EnabledState::Enabled,
            restart_policy: RestartPolicy::Always,
        }],
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    }
}

fn observed_state() -> ObservedState {
    ObservedState {
        observed_revision_id: None,
        units: Vec::new(),
        workloads: Vec::new(),
        last_reconcile_id: None,
        host_info: None,
    }
}

#[test]
fn audit_record_format_includes_plan_summary() {
    let desired = desired_state();
    let observed = observed_state();
    let plan = plan(&desired, &observed).expect("plan");

    let record = build_audit_record("run:plan", Vec::new(), &plan, Vec::new());
    let output = format_audit_record(&record);

    assert!(output.contains("plan "));
    assert!(output.contains("actions"));
}

#[test]
fn audit_event_json_is_structured() {
    let desired = desired_state();
    let observed = observed_state();
    let plan = plan(&desired, &observed).expect("plan");
    let run = core_ops::core::types::ReconcileRun {
        run_id: "run:test".to_string(),
        mode: core_ops::core::types::ReconcileMode::Plan,
        status: core_ops::core::types::RunStatus::Success,
        failure_class: None,
        summary: "planned".to_string(),
    };

    let event = build_audit_event(&run, Some(&plan), &[]);
    let json = format_audit_event_json(&event);

    assert!(json.contains("\"run_id\""));
    assert!(json.contains("\"plan_id\""));
    assert!(json.contains("\"action_count\""));
    assert!(json.contains("\"status\""));
    assert!(json.contains("\"summary\""));
}
