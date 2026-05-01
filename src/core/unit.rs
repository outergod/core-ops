use std::collections::BTreeSet;
use std::path::Path;

use crate::core::types::{MountDeclaration, ServiceDependencyEdit};

pub fn systemd_unit_for_quadlet_file(unit_file: &str) -> String {
    let path = Path::new(unit_file);
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(unit_file);
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    match ext {
        "service" => unit_file.to_string(),
        "socket" => format!("{stem}.socket"),
        "mount" => format!("{stem}.mount"),
        "automount" => format!("{stem}.automount"),
        // Native systemd unit kinds preserve their extension verbatim
        // — `backup.timer` runs as `backup.timer`, NOT `backup.service`.
        "timer" => format!("{stem}.timer"),
        "target" => format!("{stem}.target"),
        "path" => format!("{stem}.path"),
        "volume" => format!("{stem}-volume.service"),
        "network" => format!("{stem}-network.service"),
        "pod" => format!("{stem}-pod.service"),
        _ => format!("{stem}.service"),
    }
}

pub fn render_mount_unit(declaration: &MountDeclaration) -> (String, String) {
    let mut contents = String::from("[Unit]\n");
    if declaration.network_backed {
        contents.push_str("After=network-online.target\nWants=network-online.target\n");
    }
    contents.push_str("\n[Mount]\n");
    contents.push_str(&format!("What={}\n", declaration.source));
    contents.push_str(&format!("Where={}\n", declaration.target_path));
    contents.push_str(&format!("Type={}\n", declaration.fstype));
    if !declaration.mount_options.is_empty() {
        contents.push_str(&format!(
            "Options={}\n",
            declaration.mount_options.join(",")
        ));
    }
    (declaration.mount_unit_name(), contents)
}

pub fn render_automount_unit(declaration: &MountDeclaration) -> Option<(String, String)> {
    let unit_name = declaration.automount_unit_name()?;
    let contents = format!(
        "[Unit]\nAfter=network-online.target\nWants=network-online.target\n\n[Automount]\nWhere={}\n",
        declaration.target_path
    );
    Some((unit_name, contents))
}

pub fn apply_service_mount_dependencies(
    base_contents: &str,
    edit: &ServiceDependencyEdit,
) -> String {
    let mut result = base_contents.trim_end().to_string();
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result.push_str("\n# mount-dependencies: core-ops\n");

    let requires_mounts_for = join_values(&edit.requires_mounts_for);
    if !requires_mounts_for.is_empty() {
        result.push_str("[Service]\n");
        result.push_str(&format!("RequiresMountsFor={}\n", requires_mounts_for));
    }

    let after_units = join_values(&edit.after_units);
    let requires_units = join_values(&edit.requires_units);
    if !after_units.is_empty() || !requires_units.is_empty() {
        result.push_str("[Unit]\n");
        if !after_units.is_empty() {
            result.push_str(&format!("After={}\n", after_units));
        }
        if !requires_units.is_empty() {
            result.push_str(&format!("Requires={}\n", requires_units));
        }
    }

    result
}

fn join_values(values: &[String]) -> String {
    let deduped: BTreeSet<&str> = values.iter().map(String::as_str).collect();
    deduped.into_iter().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::systemd_unit_for_quadlet_file;

    #[test]
    fn timer_target_and_path_units_keep_their_native_extension() {
        // Codex P1 on PR #28: native systemd unit kinds (.timer,
        // .target, .path) MUST resolve to their own extension, not
        // fall through to the catch-all `.service` mapping. A
        // `backup.timer` workload runs as `backup.timer`; mapping it
        // to `backup.service` would query/start the wrong unit.
        assert_eq!(
            systemd_unit_for_quadlet_file("backup.timer"),
            "backup.timer"
        );
        assert_eq!(
            systemd_unit_for_quadlet_file("readiness.target"),
            "readiness.target"
        );
        assert_eq!(
            systemd_unit_for_quadlet_file("inbox.path"),
            "inbox.path"
        );
    }

    #[test]
    fn quadlet_kinds_resolve_to_their_runtime_units() {
        // Sanity guards for the existing arms — if these regress, the
        // controller queries the wrong runtime unit.
        assert_eq!(
            systemd_unit_for_quadlet_file("alpha.container"),
            "alpha.service"
        );
        assert_eq!(
            systemd_unit_for_quadlet_file("alpha.socket"),
            "alpha.socket"
        );
        assert_eq!(
            systemd_unit_for_quadlet_file("alpha.mount"),
            "alpha.mount"
        );
        assert_eq!(
            systemd_unit_for_quadlet_file("alpha.automount"),
            "alpha.automount"
        );
        assert_eq!(
            systemd_unit_for_quadlet_file("alpha.volume"),
            "alpha-volume.service"
        );
    }
}
