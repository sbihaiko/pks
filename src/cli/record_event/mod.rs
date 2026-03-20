mod helpers;
pub use helpers::{
    CAPTURED_TOOLS, build_tool_input_summary, extract_file_paths, sanitize_summary,
};

use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::hooks::hook_payload::PostToolUsePayload;
use crate::hooks::journal_entry::JournalEntry;

fn read_stdin() -> Result<String, std::io::Error> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

fn append_entry(sessions_dir: &Path, session_id: &str, line: &str) -> Result<(), std::io::Error> {
    use std::fs::OpenOptions;
    use std::io::Write;

    std::fs::create_dir_all(sessions_dir)?;
    let path = sessions_dir.join(format!("{session_id}.jsonl"));
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn build_entry(payload: &PostToolUsePayload) -> Option<JournalEntry> {
    if !CAPTURED_TOOLS.contains(&payload.tool_name.as_str()) {
        return None;
    }
    let file_paths = extract_file_paths(&payload.tool_name, &payload.tool_input);
    let raw = build_tool_input_summary(&payload.tool_name, &payload.tool_input, &file_paths);
    let tool_input_summary = sanitize_summary(&raw);
    let outcome = if payload.tool_response.success { "success" } else { "failure" };
    Some(JournalEntry {
        timestamp: Utc::now(),
        tool_name: payload.tool_name.clone(),
        tool_input_summary,
        outcome: outcome.to_owned(),
        file_paths,
        decision_note: None,
    })
}

/// Core implementation — accepts an explicit sessions directory for testability.
pub fn record_event_to_dir(sessions_dir: &Path) -> i32 {
    let raw = match read_stdin() {
        Ok(s) => s,
        Err(e) => { eprintln!("pks record-event: failed to read stdin: {e}"); return 0; }
    };

    let payload: PostToolUsePayload = match serde_json::from_str(&raw) {
        Ok(p) => p,
        Err(_) => return 0,
    };

    let entry = match build_entry(&payload) {
        Some(e) => e,
        None => return 0,
    };

    let line = match serde_json::to_string(&entry) {
        Ok(s) => s,
        Err(e) => { eprintln!("pks record-event: serialize error: {e}"); return 0; }
    };

    if let Err(e) = append_entry(sessions_dir, &payload.session_id, &line) {
        eprintln!("pks record-event: write error: {e}");
    }
    0
}

/// Entry point for `pks record-event`.
pub fn run_record_event() -> i32 {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let sessions_dir = PathBuf::from(home).join(".pks/sessions");
    record_event_to_dir(&sessions_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_sessions() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn edit_payload_str(session_id: &str, file_path: &str, success: bool) -> String {
        serde_json::json!({
            "session_id": session_id,
            "cwd": "/tmp/test-project",
            "tool_name": "Edit",
            "tool_input": { "file_path": file_path, "old_string": "a", "new_string": "b" },
            "tool_response": { "success": success }
        })
        .to_string()
    }

    #[test]
    fn extract_edit_returns_single_path() {
        let input = serde_json::json!({ "file_path": "src/main.rs" });
        assert_eq!(extract_file_paths("Edit", &input), vec!["src/main.rs"]);
    }

    #[test]
    fn extract_bash_returns_empty() {
        let input = serde_json::json!({ "command": "cargo build" });
        assert!(extract_file_paths("Bash", &input).is_empty());
    }

    #[test]
    fn build_summary_edit_uses_file_path() {
        let input = serde_json::json!({ "file_path": "src/lib.rs" });
        let paths = vec!["src/lib.rs".to_owned()];
        assert_eq!(build_tool_input_summary("Edit", &input, &paths), "src/lib.rs");
    }

    #[test]
    fn filtered_tools_not_in_captured_list() {
        for tool in &["Read", "Glob", "Grep"] {
            assert!(!CAPTURED_TOOLS.contains(tool), "{tool} must be filtered");
        }
    }

    #[test]
    fn integration_writes_jsonl_for_edit() {
        let tmp = tmp_sessions();
        let sessions_dir = tmp.path();
        let payload: PostToolUsePayload =
            serde_json::from_str(&edit_payload_str("sess-x", "src/main.rs", true)).unwrap();

        let entry = build_entry(&payload).expect("Edit should produce an entry");
        let line = serde_json::to_string(&entry).unwrap();
        append_entry(sessions_dir, &payload.session_id, &line).unwrap();

        let out = sessions_dir.join("sess-x.jsonl");
        assert!(out.exists());
        let contents = std::fs::read_to_string(&out).unwrap();
        let parsed: JournalEntry = serde_json::from_str(contents.trim()).unwrap();
        assert_eq!(parsed.tool_name, "Edit");
        assert_eq!(parsed.outcome, "success");
        assert_eq!(parsed.file_paths, vec!["src/main.rs"]);
    }

    #[test]
    fn integration_appends_multiple_lines() {
        let tmp = tmp_sessions();
        for i in 0..3_u8 {
            append_entry(tmp.path(), "sess-m", &format!("{{\"i\":{i}}}")).unwrap();
        }
        let contents = std::fs::read_to_string(tmp.path().join("sess-m.jsonl")).unwrap();
        assert_eq!(contents.lines().count(), 3);
    }

    #[test]
    fn secrets_redacted_in_sanitize_summary() {
        let raw = "uses sk-abc123def456ghi789jkl0 token";
        let out = sanitize_summary(raw);
        assert!(!out.contains("sk-abc123"));
        assert!(out.contains("[REDACTED_API_KEY]"));
    }
}
