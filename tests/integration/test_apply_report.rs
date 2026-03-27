use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::cli::apply::apply_with_report;
use serde_json::Value;
use crate::integration::env_lock::path_lock;

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
fn apply_report_includes_diffs_and_actions() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_apply_report");
    let rev = init_git_repo(&repo);

    let systemctl = temp_dir("core_ops_systemctl_apply_report");
    fs::create_dir_all(&systemctl).expect("systemctl dir");
    let log_path = systemctl.join("systemctl.log");
    write_systemctl_stub(&systemctl, &log_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", systemctl.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp_dir("core_ops_host_apply_report");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let (result, report, _plan) =
        apply_with_report(repo.to_str().unwrap(), &rev, &host_quadlets, false, None)
            .expect("apply report");

    assert!(report.contains("diffs"));
    assert!(report.contains("actions"));
    let json_line = report
        .lines()
        .find(|line| line.starts_with('{'))
        .expect("structured apply report json");
    let parsed: Value = serde_json::from_str(json_line).expect("parse apply json");
    let expected_status = match result.run.status {
        core_ops::core::types::RunStatus::Success => "success",
        core_ops::core::types::RunStatus::Failure => "failure",
    };
    assert_eq!(parsed["status"].as_str(), Some(expected_status));
    assert_eq!(parsed["summary"].as_str(), Some(result.run.summary.as_str()));
    assert!(parsed["verification_results"].is_array());
    assert!(parsed["convergence"]["status"].is_string());
}

#[test]
#[ignore = "apply/result public contract lands in later phases"]
fn apply_result_parity_contract_scaffolding_is_reserved() {
    assert!(true);
}
