use std::path::{Path, PathBuf};

use crate::io::state::{parse_persisted_state_text, read_persisted_state, resolve_state_file};

pub fn render_status_from_path(path: &Path) -> String {
    match read_persisted_state(path) {
        Ok(Some(state)) => render_present_state(&state),
        Ok(None) | Err(_) => absent_status(),
    }
}

pub fn render_status(explicit: Option<PathBuf>) -> String {
    let path = resolve_state_file(explicit);
    render_status_from_path(&path)
}

pub fn format_status_text(contents: &str) -> String {
    match parse_persisted_state_text(contents) {
        Some(state) => render_present_state(&state),
        None => absent_status(),
    }
}

fn render_present_state(state: &crate::core::types::PersistedProvenanceState) -> String {
    match serde_json::to_string_pretty(state) {
        Ok(pretty) => format!("provenance\n{}", pretty),
        Err(_) => absent_status(),
    }
}

fn absent_status() -> String {
    "provenance\n{\n  \"status\": \"absent\"\n}".to_string()
}
