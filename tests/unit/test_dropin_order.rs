use core_ops::core::evaluate::evaluate_desired_state;
use core_ops::core::types::{
    ArtifactSource, DropInSource, EvaluationInput, HostDeclaration, HostOverlaySet, QuadletType,
    ServiceCatalog, ServiceDefinition,
};
use std::collections::BTreeMap;
#[test]
fn applies_dropins_in_lexicographic_order_with_host_overrides_last() {
    let service = ServiceDefinition {
        name: "alpha".to_string(),
        artifacts: vec![ArtifactSource {
            name: "alpha.container".to_string(),
            quadlet_type: QuadletType::Container,
            contents: "BASE".to_string(),
            source_path: "/services/alpha/alpha.container".to_string(),
        }],
        base_dropins: vec![
            DropInSource {
                target: "alpha.container".to_string(),
                contents: "B".to_string(),
                source_path: "/services/alpha/alpha.container.d/20-b.conf".to_string(),
            },
            DropInSource {
                target: "alpha.container".to_string(),
                contents: "A".to_string(),
                source_path: "/services/alpha/alpha.container.d/10-a.conf".to_string(),
            },
        ],
        config_files: Vec::new(),
    };

    let mut services = BTreeMap::new();
    services.insert("alpha".to_string(), service);

    let input = EvaluationInput {
        host: HostDeclaration {
            host: "kadath".to_string(),
            services: vec!["alpha".to_string()],
        },
        catalog: ServiceCatalog { services },
        overlays: HostOverlaySet {
            host: "kadath".to_string(),
            overrides: vec![DropInSource {
                target: "alpha.container".to_string(),
                contents: "HOST".to_string(),
                source_path: "/hosts/kadath/overrides/alpha.container.d/20-host.conf".to_string(),
            }],
            config_overrides: Vec::new(),
        },
    };

    let output = evaluate_desired_state(&input).expect("evaluate");
    let artifact = &output.artifacts[0];

    assert_eq!(artifact.contents, "BASE\nA\nB\nHOST");
    assert_eq!(
        artifact.source_layers,
        vec![
            "/services/alpha/alpha.container",
            "/services/alpha/alpha.container.d/10-a.conf",
            "/services/alpha/alpha.container.d/20-b.conf",
            "/hosts/kadath/overrides/alpha.container.d/20-host.conf",
        ]
    );
}
