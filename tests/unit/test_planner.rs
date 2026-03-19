use core_ops::core::planner::plan;
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, ObservedState,
    QuadletType, RestartPolicy, Workload,
};

fn workload(name: &str) -> Workload {
    Workload {
        name: name.to_string(),
        quadlet_type: QuadletType::Container,
        quadlet_contents: "[Container]".to_string(),
        systemd_unit_name: format!("{}.container", name),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn desired_state(workloads: Vec<Workload>) -> DesiredState {
    DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads,
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    }
}

fn observed_state(workloads: Vec<Workload>) -> ObservedState {
    ObservedState {
        observed_revision_id: Some("obs".to_string()),
        units: Vec::new(),
        workloads,
        last_reconcile_id: None,
        host_info: None,
    }
}

#[test]
fn plan_is_deterministic_by_name_order() {
    let desired = desired_state(vec![workload("beta"), workload("alpha")]);
    let observed = observed_state(Vec::new());

    let plan = plan(&desired, &observed).expect("plan should succeed");

    let targets: Vec<String> = plan.actions.iter().map(|a| a.target.clone()).collect();

    let alpha_prefix = vec![
        "alpha".to_string(),
        "alpha".to_string(),
        "alpha".to_string(),
    ];
    assert_eq!(&targets[..3], &alpha_prefix[..]);
}

#[test]
fn plan_has_no_actions_when_states_match() {
    let workloads = vec![workload("alpha")];
    let desired = desired_state(workloads.clone());
    let observed = observed_state(workloads);

    let plan = plan(&desired, &observed).expect("plan should succeed");

    assert!(plan.actions.is_empty());
}
