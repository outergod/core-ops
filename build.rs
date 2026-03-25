use std::process::Command;

fn main() {
    emit_git_revision();
    emit_build_time();
    emit_tree_state();
}

fn emit_git_revision() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    if let Some(revision) = run_command(&["git", "rev-parse", "--short", "HEAD"]) {
        println!("cargo:rustc-env=CORE_OPS_BUILD_REVISION={revision}");
    }
}

fn emit_build_time() {
    if let Ok(source_date_epoch) = std::env::var("SOURCE_DATE_EPOCH") {
        if !source_date_epoch.trim().is_empty() {
            println!("cargo:rustc-env=CORE_OPS_BUILD_TIME={source_date_epoch}");
            return;
        }
    }

    if let Some(build_time) = run_command(&["date", "-u", "+%Y-%m-%dT%H:%M:%SZ"]) {
        println!("cargo:rustc-env=CORE_OPS_BUILD_TIME={build_time}");
    }
}

fn emit_tree_state() {
    let tree_state = match Command::new("git")
        .args(["diff", "--quiet", "--ignore-submodules=dirty"])
        .status()
    {
        Ok(status) if status.success() => Some("clean"),
        Ok(status) if status.code() == Some(1) => Some("dirty"),
        _ => None,
    };

    if let Some(tree_state) = tree_state {
        println!("cargo:rustc-env=CORE_OPS_TREE_STATE={tree_state}");
    }
}

fn run_command(args: &[&str]) -> Option<String> {
    let (program, rest) = args.split_first()?;
    let output = Command::new(program).args(rest).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}
