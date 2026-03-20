use std::fs;
use std::path::Path;

#[test]
fn quickstart_mentions_systemd_units_and_env() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let quickstart = root.join("specs/002-systemd-agent/quickstart.md");
    let contents = fs::read_to_string(&quickstart).expect("read quickstart");

    assert!(contents.contains("core-ops.service"));
    assert!(contents.contains("core-ops.timer"));
    assert!(contents.contains("CORE_OPS_REPO"));
    assert!(contents.contains("CORE_OPS_REV"));
}
