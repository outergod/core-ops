use std::path::Path;

pub fn systemd_unit_for_quadlet_file(unit_file: &str) -> String {
    let path = Path::new(unit_file);
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(unit_file);
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    match ext {
        "socket" => format!("{stem}.socket"),
        _ => format!("{stem}.service"),
    }
}
