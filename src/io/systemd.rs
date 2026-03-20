use std::path::PathBuf;

pub const SYSTEMD_UNIT_DIR_ENV: &str = "CORE_OPS_SYSTEMD_UNIT_DIR";

pub fn systemd_unit_dir() -> PathBuf {
    std::env::var_os(SYSTEMD_UNIT_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/systemd/system"))
}
