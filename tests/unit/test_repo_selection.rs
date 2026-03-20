use std::path::Path;

use core_ops::io::repo::load_host_declaration;

#[test]
fn loads_host_declaration_when_dir_and_field_match() {
    let temp = tempfile::tempdir().expect("tempdir");
    let host_dir = temp.path().join("kadath");
    std::fs::create_dir_all(&host_dir).expect("host dir");
    std::fs::write(
        host_dir.join("host.yaml"),
        "host: kadath\nservices:\n  - traefik\n",
    )
    .expect("write host.yaml");

    let host = load_host_declaration(Path::new(&host_dir)).expect("load host declaration");

    assert_eq!(host.host, "kadath");
    assert_eq!(host.services, vec!["traefik".to_string()]);
}

#[test]
fn rejects_host_declaration_when_host_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let host_dir = temp.path().join("ulthar");
    std::fs::create_dir_all(&host_dir).expect("host dir");
    std::fs::write(
        host_dir.join("host.yaml"),
        "host: kadath\nservices:\n  - traefik\n",
    )
    .expect("write host.yaml");

    let err = load_host_declaration(Path::new(&host_dir)).expect_err("should fail");

    assert!(format!("{err}").contains("does not match"));
}
