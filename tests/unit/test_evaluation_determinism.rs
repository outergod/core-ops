use core_ops::core::evaluate::evaluate_desired_state;
use core_ops::core::types::{
    ArtifactSource, DropInSource, EvaluationInput, HostDeclaration, HostOverlaySet, QuadletType,
    ServiceCatalog, ServiceDefinition,
};
use std::collections::BTreeMap;

#[test]
fn evaluation_is_deterministic_for_same_input() {
    let service = ServiceDefinition {
        name: "alpha".to_string(),
        artifacts: vec![
            ArtifactSource {
                name: "beta.container".to_string(),
                quadlet_type: QuadletType::Container,
                contents: "B".to_string(),
                source_path: "/services/alpha/beta.container".to_string(),
            },
            ArtifactSource {
                name: "alpha.container".to_string(),
                quadlet_type: QuadletType::Container,
                contents: "A".to_string(),
                source_path: "/services/alpha/alpha.container".to_string(),
            },
        ],
        base_dropins: vec![DropInSource {
            target: "alpha.container".to_string(),
            contents: "X".to_string(),
            source_path: "/services/alpha/alpha.container.d/10-x.conf".to_string(),
        }],
    };

    let mut services = BTreeMap::new();
    services.insert("alpha".to_string(), service);

    let input = EvaluationInput {
        host: HostDeclaration {
            host: "ulthar".to_string(),
            services: vec!["alpha".to_string()],
        },
        catalog: ServiceCatalog { services },
        overlays: HostOverlaySet {
            host: "ulthar".to_string(),
            overrides: vec![DropInSource {
                target: "alpha.container".to_string(),
                contents: "Y".to_string(),
                source_path: "/hosts/ulthar/overrides/alpha.container.d/20-y.conf".to_string(),
            }],
        },
    };

    let first = evaluate_desired_state(&input).expect("evaluate");
    let second = evaluate_desired_state(&input).expect("evaluate");

    assert_eq!(first, second);
    assert_eq!(first.artifacts[0].name, "alpha.container");
    assert_eq!(first.artifacts[1].name, "beta.container");
}
