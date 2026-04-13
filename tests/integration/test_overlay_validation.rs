use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use core_ops::io::repo::load_desired_state;

struct HostGuard(Option<std::ffi::OsString>);

impl Drop for HostGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.0 {
            std::env::set_var("CORE_OPS_HOST", value);
        } else {
            std::env::remove_var("CORE_OPS_HOST");
        }
    }
}

fn temp_repo() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("core_ops_overlay_invalid_{}", nanos));
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

    let invalid_dir = repo
        .join("hosts")
        .join("kadath")
        .join("overrides")
        .join("missing.container.d");
    std::fs::create_dir_all(&invalid_dir).expect("invalid dir");
    std::fs::write(
        invalid_dir.join("10-invalid.conf"),
        "[Container]\nImage=bad",
    )
    .expect("write invalid drop-in");

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
fn fails_on_dropin_target_missing() {
    let _lock = path_lock().lock().expect("path lock");
    let previous_host = std::env::var_os("CORE_OPS_HOST");
    let _host_guard = HostGuard(previous_host);
    let repo = temp_repo();
    let rev = init_layered_repo(&repo);

    std::env::set_var("CORE_OPS_HOST", "kadath");
    let err = load_desired_state(repo.to_str().unwrap(), &rev).expect_err("should fail");

    assert!(err.to_string().contains("drop-in target does not exist"));
    // CORE_OPS_HOST is restored by _host_guard on drop
}
