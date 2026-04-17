use std::path::Path;

#[test]
fn systemd_unit_templates_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let service = root.join("systemd/core-ops.service");
    let timer = root.join("systemd/core-ops.timer");

    assert!(
        service.exists(),
        "missing service template: {}",
        service.display()
    );
    assert!(
        timer.exists(),
        "missing timer template: {}",
        timer.display()
    );
}

#[test]
fn systemd_service_does_not_reference_removed_flags() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let contents = std::fs::read_to_string(root.join("systemd/core-ops.service"))
        .expect("read core-ops.service");

    assert!(
        !contents.contains("--repo"),
        "core-ops.service must not reference removed --repo flag"
    );
    assert!(
        !contents.contains("--rev"),
        "core-ops.service must not reference removed --rev flag"
    );
    assert!(
        contents.contains("core-ops agent"),
        "core-ops.service ExecStart must invoke 'core-ops agent'"
    );
}
