use crate::hooks::journal_entry::{redact_secrets, truncate_summary};

/// Tools whose events are captured and journaled.
pub const CAPTURED_TOOLS: &[&str] = &["Edit", "Write", "MultiEdit", "Bash"];

/// Extracts file paths from `tool_input` based on the tool name.
pub fn extract_file_paths(tool_name: &str, tool_input: &serde_json::Value) -> Vec<String> {
    match tool_name {
        "Edit" | "Write" => {
            tool_input["file_path"]
                .as_str()
                .map(|s| vec![s.to_owned()])
                .unwrap_or_default()
        }
        "MultiEdit" => extract_multi_edit_paths(tool_input),
        _ => vec![],
    }
}

fn extract_multi_edit_paths(tool_input: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = tool_input["edits"].as_array() {
        return arr
            .iter()
            .filter_map(|e| e["file_path"].as_str().map(str::to_owned))
            .collect();
    }
    tool_input["file_path"]
        .as_str()
        .map(|s| vec![s.to_owned()])
        .unwrap_or_default()
}

/// Builds a short human-readable summary of the tool input.
pub fn build_tool_input_summary(
    tool_name: &str,
    tool_input: &serde_json::Value,
    file_paths: &[String],
) -> String {
    match tool_name {
        "Edit" | "Write" => tool_input["file_path"]
            .as_str()
            .unwrap_or("(unknown)")
            .to_owned(),
        "Bash" => tool_input["command"]
            .as_str()
            .unwrap_or("bash command")
            .to_owned(),
        "MultiEdit" => {
            if file_paths.is_empty() { "multi-edit".to_owned() } else { file_paths.join(", ") }
        }
        _ => tool_name.to_owned(),
    }
}

/// Applies truncation and secret redaction to a raw summary string.
pub fn sanitize_summary(raw: &str) -> String {
    redact_secrets(truncate_summary(raw, 200))
}
