use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::io::repo::load_desired_state;

fn temp_repo() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("core_ops_layered_{}", nanos));
    path
}

fn copy_dir_all(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("create dir");
    for entry in std::fs::read_dir(src).expect("read dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &dest);
        } else {
            std::fs::copy(&path, &dest).expect("copy file");
        }
    }
}

fn init_layered_repo(repo: &PathBuf) -> String {
    std::process::Command::new("git")
        .arg("init")
        .arg(repo)
        .output()
        .expect("git init");

    let fixtures = PathBuf::from("tests/fixtures/layered_overrides");
    copy_dir_all(&fixtures.join("services"), &repo.join("services"));
    copy_dir_all(&fixtures.join("hosts"), &repo.join("hosts"));

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
fn selects_services_per_host() {
    let repo = temp_repo();
    let rev = init_layered_repo(&repo);

    std::env::set_var("CORE_OPS_HOST", "kadath");
    let desired = load_desired_state(repo.to_str().unwrap(), &rev).expect("load desired");
    let names: Vec<_> = desired
        .workloads
        .iter()
        .map(|w| w.systemd_unit_name.as_str())
        .collect();

    assert!(names.contains(&"traefik.container"));
    assert!(names.contains(&"traefik.socket"));
    assert!(names.contains(&"immich.container"));
    assert!(names.contains(&"immich.volume"));
    assert!(!names.contains(&"vector.container"));
    assert!(!names.contains(&"whoami.container"));

    std::env::set_var("CORE_OPS_HOST", "rlyeh");
    let desired = load_desired_state(repo.to_str().unwrap(), &rev).expect("load desired");
    let names: Vec<_> = desired
        .workloads
        .iter()
        .map(|w| w.systemd_unit_name.as_str())
        .collect();

    assert!(names.contains(&"traefik.container"));
    assert!(names.contains(&"traefik.socket"));
    assert!(names.contains(&"vector.container"));
    assert!(!names.contains(&"immich.container"));
    assert!(!names.contains(&"immich.volume"));

    std::env::remove_var("CORE_OPS_HOST");
}
