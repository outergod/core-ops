use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::io::repo::load_desired_state;

fn temp_repo() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("core_ops_repo_{}", nanos));
    path
}

#[test]
fn loads_desired_state_from_quadlet_dir() {
    let repo = temp_repo();
    let quadlets = repo.join("quadlets");
    fs::create_dir_all(&quadlets).expect("create quadlets");

    let file_path = quadlets.join("alpha.container");
    fs::write(&file_path, "[Container]\nImage=alpine").expect("write file");

    let desired = load_desired_state(&repo, "rev-1").expect("load desired");

    assert_eq!(desired.revision_id, "rev-1");
    assert_eq!(desired.workloads.len(), 1);
    assert_eq!(desired.workloads[0].name, "alpha");
    assert_eq!(desired.workloads[0].systemd_unit_name, "alpha.container");
}
