use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::{reconcile_apply, ReconcileDependencies};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::read_observed_state;
use core_ops::io::repo::load_desired_state;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
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

fn write_systemctl_stub(dir: &PathBuf, log_path: &PathBuf) -> PathBuf {
    let bin_path = dir.join("systemctl");
    let script = format!(
        "#!/bin/sh\n\n\
echo \"$@\" >> \"{}\"\n\
exit 0\n",
        log_path.display()
    );
    fs::write(&bin_path, script).expect("write systemctl stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin_path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).expect("chmod");
    }
    bin_path
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}

#[test]
fn reconcile_apply_converges_to_desired_state() {
    let repo = temp_dir("core_ops_repo");
    let rev = init_git_repo(&repo);

    let temp = temp_dir("core_ops_systemctl");
    fs::create_dir_all(&temp).expect("systemctl temp");
    let log_path = temp.join("systemctl.log");
    write_systemctl_stub(&temp, &log_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp_dir("core_ops_host");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(repo.to_str().unwrap(), &rev).map_err(map_io_error),
        read_observed: &|| read_observed_state(&host_quadlets, Some("obs".to_string()))
            .map_err(map_io_error),
        apply_plan: &|plan, desired| {
            apply_plan(plan, &desired.workloads, &host_quadlets, false)
                .map(|_| ())
                .map_err(map_io_error)
        },
    };

    let run = reconcile_apply(&deps).expect("reconcile apply");

    assert_eq!(run.summary, "converged");
}

fn map_io_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError {
        class: core_ops::core::types::FailureClass::Apply,
        message: err.to_string(),
    }
}
