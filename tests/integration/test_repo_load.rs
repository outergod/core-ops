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

fn init_git_repo(repo: &PathBuf) -> String {
    std::process::Command::new("git")
        .arg("init")
        .arg(repo)
        .output()
        .expect("git init");

    let quadlets = repo.join("quadlets");
    std::fs::create_dir_all(&quadlets).expect("create quadlets");
    std::fs::write(quadlets.join("alpha.container"), "[Container]\nImage=alpine")
        .expect("write quadlet");

    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("add")
        .arg(".")
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("commit")
        .arg("-m")
        .arg("fixture")
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
        .output()
        .expect("git commit");

    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn loads_desired_state_from_quadlet_dir() {
    let repo = temp_repo();
    let rev = init_git_repo(&repo);

    let desired = load_desired_state(repo.to_str().unwrap(), &rev).expect("load desired");

    assert_eq!(desired.revision_id, rev);
    assert_eq!(desired.workloads.len(), 1);
    assert_eq!(desired.workloads[0].name, "alpha");
    assert_eq!(desired.workloads[0].systemd_unit_name, "alpha.container");
}

#[test]
fn loads_desired_state_from_git_url_fixture() {
    let repo = temp_repo();
    let rev = init_git_repo(&repo);

    let repo_url = format!("file://{}", repo.display());
    let desired = load_desired_state(&repo_url, &rev).expect("load desired");

    assert!(!desired.workloads.is_empty());
}
