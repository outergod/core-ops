use std::path::Path;

#[test]
fn systemd_unit_templates_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let service = root.join("specs/002-systemd-agent/contracts/systemd/core-ops.service");
    let timer = root.join("specs/002-systemd-agent/contracts/systemd/core-ops.timer");

    assert!(service.exists(), "missing service template: {}", service.display());
    assert!(timer.exists(), "missing timer template: {}", timer.display());
}
