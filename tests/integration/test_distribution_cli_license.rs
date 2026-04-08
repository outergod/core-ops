use std::process::Command;

#[test]
fn core_ops_help_mentions_governing_license() {
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops"))
        .arg("--help")
        .output()
        .expect("run core-ops --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("GNU Affero General Public License version 3 or later"));
    assert!(stdout.contains("AGPLv3+"));
}
