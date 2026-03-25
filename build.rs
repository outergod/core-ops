use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rerun-if-env-changed=CORE_OPS_BUILD_REVISION");
    println!("cargo:rerun-if-env-changed=CORE_OPS_BUILD_TIME");
    println!("cargo:rerun-if-env-changed=CORE_OPS_TREE_STATE");

    let revision = env_or_git_revision();
    let build_time = env_or_build_time();
    let tree_state = env_or_tree_state().unwrap_or_else(|| "unknown".to_string());

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let contents = format!(
        "pub const BUILD_REVISION: Option<&str> = {revision};\n\
         pub const BUILD_TIME: Option<&str> = {build_time};\n\
         pub const BUILD_TREE_STATE: &str = {tree_state:?};\n",
        revision = option_literal(revision.as_deref()),
        build_time = option_literal(build_time.as_deref()),
        tree_state = tree_state,
    );
    fs::write(out_dir.join("build_info.rs"), contents).expect("write build_info.rs");
}

fn env_or_git_revision() -> Option<String> {
    std::env::var("CORE_OPS_BUILD_REVISION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| run_command(&["git", "rev-parse", "--short", "HEAD"]))
}

fn env_or_build_time() -> Option<String> {
    std::env::var("CORE_OPS_BUILD_TIME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| run_command(&["date", "-u", "+%Y-%m-%dT%H:%M:%SZ"]))
}

fn env_or_tree_state() -> Option<String> {
    std::env::var("CORE_OPS_TREE_STATE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            match Command::new("git")
                .args(["diff", "--quiet", "--ignore-submodules=dirty"])
                .status()
            {
                Ok(status) if status.success() => Some("clean".to_string()),
                Ok(status) if status.code() == Some(1) => Some("dirty".to_string()),
                _ => None,
            }
        })
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

fn option_literal(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("Some({value:?})"),
        None => "None".to_string(),
    }
}
