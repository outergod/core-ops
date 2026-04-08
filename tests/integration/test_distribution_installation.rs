use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn readme_documents_binary_installation_sequence() {
    let contents = fs::read_to_string(repo_root().join("README.md")).expect("read README");

    assert!(contents.contains("## Installation (Current Phase)"));
    assert!(contents.contains("x86_64"));
    assert!(contents.contains("aarch64"));
    assert!(contents.contains("core-ops-linux-<arch>.tar.gz"));
    assert!(contents.contains("install -m 0755 core-ops-linux-<arch> /usr/local/bin/core-ops"));
    assert!(contents.contains("install -m 0644 core-ops.service /etc/systemd/system/core-ops.service"));
    assert!(contents.contains("install -m 0644 core-ops.timer /etc/systemd/system/core-ops.timer"));
    assert!(contents.contains("core-ops.service"));
    assert!(contents.contains("core-ops.timer"));
    assert!(contents.contains("systemctl enable core-ops.service"));
    assert!(contents.contains("systemctl enable --now core-ops.timer"));
}

#[test]
fn quickstart_documents_cold_start_supported_environment_flow() {
    let contents = fs::read_to_string(
        repo_root().join("specs/010-distribution-readiness/quickstart.md"),
    )
    .expect("read quickstart");

    assert!(contents.contains("Fresh Fedora CoreOS system"));
    assert!(contents.contains("No undeclared host preparation"));
    assert!(contents.contains("Run the documented first command"));
    assert!(contents.contains("core-ops.service"));
    assert!(contents.contains("core-ops.timer"));
    assert!(contents.contains("including the canonical `core-ops.service` and"));
    assert!(contents.contains("Execute the documented minimal operator verification flow"));
}
