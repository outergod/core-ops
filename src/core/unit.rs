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
        "socket" => format!("{stem}.socket"),
        "mount" => format!("{stem}.mount"),
        "automount" => format!("{stem}.automount"),
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
        contents.push_str(&format!("Options={}\n", declaration.mount_options.join(",")));
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

pub fn apply_service_mount_dependencies(base_contents: &str, edit: &ServiceDependencyEdit) -> String {
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
