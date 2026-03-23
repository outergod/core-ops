pub fn format_status_text(contents: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(contents) {
        Ok(value) => match serde_json::to_string_pretty(&value) {
            Ok(pretty) => format!("provenance\n{}", pretty),
            Err(_) => format!("provenance\n{}", contents.trim_end()),
        },
        Err(_) => format!("provenance\n{}", contents.trim_end()),
    }
}
