use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use core_ops::io::repo::load_desired_state;

fn temp_repo() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("core_ops_mount_reuse_{}", nanos));
    path
}

fn init_repo(repo: &Path) -> String {
    // Container goes under quadlet/, mount/automount under systemd/ per
    // the formalized payload-kind split. Host overlays target the
    // per-service tree at hosts/<host>/<svc>/systemd/<unit>.<ext>.d/.
    let svc_quadlet = repo.join("services/immich/quadlet");
    let svc_systemd = repo.join("services/immich/systemd");
    std::fs::create_dir_all(&svc_quadlet).expect("service quadlet dir");
    std::fs::create_dir_all(&svc_systemd).expect("service systemd dir");
    std::fs::create_dir_all(repo.join("hosts/alpha")).expect("host alpha");
    std::fs::create_dir_all(
        repo.join("hosts/beta/immich/systemd/var-lib-immich-media.mount.d"),
    )
    .expect("beta mount overrides");
    std::fs::create_dir_all(
        repo.join("hosts/beta/immich/systemd/var-lib-immich-media.automount.d"),
    )
    .expect("beta automount overrides");
    std::fs::create_dir_all(
        repo.join("hosts/invalid/immich/systemd/var-lib-immich-media.mount.d"),
    )
    .expect("host invalid");

    std::fs::write(
        svc_quadlet.join("immich.container"),
        "[Container]\nImage=immich\n[Service]\nRequiresMountsFor=/var/lib/immich/media\n[Unit]\nAfter=var-lib-immich-media.automount var-lib-immich-media.mount\nRequires=var-lib-immich-media.automount var-lib-immich-media.mount\n",
    )
    .expect("write artifact");
    std::fs::write(
        svc_systemd.join("var-lib-immich-media.mount"),
        r#"[Unit]
After=network-online.target
Wants=network-online.target

[Mount]
What=nas:/volume1/media
Where=/var/lib/immich/media
Type=nfs
Options=rw,hard

[X-CoreOps]
CreateMountpoint=true
"#,
    )
    .expect("write media mount");
    std::fs::write(
        svc_systemd.join("var-lib-immich-media.automount"),
        "[Automount]\nWhere=/var/lib/immich/media\n",
    )
    .expect("write media automount");
    std::fs::write(
        svc_systemd.join("var-lib-immich-cache.mount"),
        r#"[Mount]
What=/var/cache/immich
Where=/var/lib/immich/cache
Type=none
Options=bind

[X-CoreOps]

"#,
    )
    .expect("write cache mount");

    for host in ["alpha", "beta", "invalid"] {
        std::fs::write(
            repo.join(format!("hosts/{host}/host.yaml")),
            format!("host: {host}\nservices:\n  - immich\n"),
        )
        .expect("write host");
    }

    std::fs::write(
        repo.join("hosts/beta/immich/systemd/var-lib-immich-media.mount.d/20-host.conf"),
        r#"[Mount]
What=nas:/volume2/media
Options=rw,hard,noatime
"#,
    )
    .expect("write beta mount drop-in");
    std::fs::write(
        repo.join("hosts/beta/immich/systemd/var-lib-immich-media.automount.d/20-host.conf"),
        "[Automount]\nWhere=/var/lib/immich/media\n",
    )
    .expect("write beta automount drop-in");

    std::fs::write(
        repo.join("hosts/invalid/immich/systemd/var-lib-immich-media.mount.d/20-host.conf"),
        r#"[Mount]
Where=/srv/immich/media
"#,
    )
    .expect("write invalid overrides");

    crate::integration::source_repo_support::git_init_commit(repo)
}

#[test]
fn reusable_mount_declarations_support_layered_native_overrides_and_service_dependencies() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let repo = temp_repo();
    let rev = init_repo(&repo);

    let previous = std::env::var_os("CORE_OPS_HOST");
    let _guard = HostGuard(previous);

    std::env::set_var("CORE_OPS_HOST", "alpha");
    let alpha = load_desired_state(repo.to_str().expect("repo str"), &rev).expect("load alpha");
    assert!(alpha
        .workloads
        .iter()
        .any(|workload| workload.systemd_unit_name == "var-lib-immich-media.mount"));
    assert!(alpha
        .workloads
        .iter()
        .any(|workload| workload.systemd_unit_name == "var-lib-immich-media.automount"));
    let alpha_service = alpha
        .workloads
        .iter()
        .find(|workload| workload.systemd_unit_name == "immich.container")
        .expect("alpha service");
    assert!(alpha_service
        .quadlet_contents
        .contains("RequiresMountsFor=/var/lib/immich/media"));
    assert!(alpha_service
        .quadlet_contents
        .contains("After=var-lib-immich-media.automount var-lib-immich-media.mount"));

    std::env::set_var("CORE_OPS_HOST", "beta");
    let beta = load_desired_state(repo.to_str().expect("repo str"), &rev).expect("load beta");
    let beta_mount = beta
        .mount_declarations
        .iter()
        .find(|mount| mount.id == "var-lib-immich-media")
        .expect("beta mount");
    assert_eq!(beta_mount.target_path, "/var/lib/immich/media");
    assert_eq!(beta_mount.source, "nas:/volume2/media");
    let beta_service = beta
        .workloads
        .iter()
        .find(|workload| workload.systemd_unit_name == "immich.container")
        .expect("beta service");
    assert!(beta_service
        .quadlet_contents
        .contains("RequiresMountsFor=/var/lib/immich/media"));
    assert!(beta_service
        .quadlet_contents
        .contains("After=var-lib-immich-media.automount var-lib-immich-media.mount"));

    std::env::set_var("CORE_OPS_HOST", "invalid");
    let err =
        load_desired_state(repo.to_str().expect("repo str"), &rev).expect_err("invalid override");
    assert!(err
        .to_string()
        .contains("mount unit name does not match Mount Where"));
}

#[test]
fn managed_mount_artifacts_reject_removed_x_coreops_fields() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let repo = temp_repo();

    let svc_quadlet = repo.join("services/immich/quadlet");
    let svc_systemd = repo.join("services/immich/systemd");
    std::fs::create_dir_all(&svc_quadlet).expect("service quadlet dir");
    std::fs::create_dir_all(&svc_systemd).expect("service systemd dir");
    std::fs::create_dir_all(repo.join("hosts/alpha")).expect("host dir");
    std::fs::write(
        svc_quadlet.join("immich.container"),
        "[Container]\nImage=immich\n[Service]\nRequiresMountsFor=/var/lib/immich/media\n",
    )
    .expect("write artifact");
    std::fs::write(
        svc_systemd.join("var-lib-immich-media.mount"),
        r#"[Mount]
What=nas:/volume1/media
Where=/var/lib/immich/media
Type=nfs

[X-CoreOps]
Id=immich-media
"#,
    )
    .expect("write mount");
    std::fs::write(
        repo.join("hosts/alpha/host.yaml"),
        "host: alpha\nservices:\n  - immich\n",
    )
    .expect("write host");

    let rev = crate::integration::source_repo_support::git_init_commit(&repo);

    let previous = std::env::var_os("CORE_OPS_HOST");
    let _guard = HostGuard(previous);
    std::env::set_var("CORE_OPS_HOST", "alpha");

    let err =
        load_desired_state(repo.to_str().expect("repo str"), &rev).expect_err("invalid field");
    assert!(err.to_string().contains("unsupported X-CoreOps field"));
}

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
