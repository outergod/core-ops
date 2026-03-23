pub fn format_status_text(contents: &str) -> String {
    format!("last run\n{}", contents.trim_end())
}
