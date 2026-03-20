use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::git::BareCommit;
use crate::hooks::journal_entry::JournalEntry;
use crate::hooks::shadow_journal::ShadowJournalHook;
use crate::ipc::{IpcClient, PksCommand};

fn try_ipc_refresh() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    if let Ok(rt) = rt {
        let _ = rt.block_on(IpcClient::send_command(&PksCommand::Refresh { dry_run: false }));
    }
}

fn parse_entries(content: &str) -> Vec<JournalEntry> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| match serde_json::from_str::<JournalEntry>(line) {
            Ok(e) => Some(e),
            Err(e) => {
                eprintln!("pks flush-session: skipping invalid JSONL line: {e}");
                None
            }
        })
        .collect()
}

fn min_words_threshold() -> usize {
    std::env::var("PKS_JOURNAL_MIN_WORDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10)
}

fn count_words(entries: &[JournalEntry]) -> usize {
    entries
        .iter()
        .flat_map(|e| e.tool_input_summary.split_whitespace())
        .count()
}

fn load_entries(jsonl_path: &Path) -> Option<Vec<JournalEntry>> {
    let content = match std::fs::read_to_string(jsonl_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("pks flush-session: failed to read {}: {e}", jsonl_path.display()); return None; }
    };
    let entries = parse_entries(&content);
    if entries.is_empty() || count_words(&entries) < min_words_threshold() {
        let _ = std::fs::remove_file(jsonl_path);
        return None;
    }
    Some(entries)
}

fn flush_entries(session_id: &str, cwd: &Path, entries: Vec<JournalEntry>) {
    let first_ts = entries.first().map(|e| e.timestamp).unwrap_or_else(Utc::now);
    let hook = ShadowJournalHook::from_entries(cwd.to_path_buf(), session_id.to_string(), first_ts, entries);
    let bc = BareCommit::new(cwd);
    if let Err(e) = hook.flush_to_vault(&bc) {
        eprintln!("pks flush-session: flush_to_vault failed: {e}");
    }
}

/// Core logic for flush-session — accepts an explicit sessions_dir for testability.
pub fn flush_session_with_dir(session_id: &str, cwd: &Path, sessions_dir: &Path) -> i32 {
    let jsonl_path = sessions_dir.join(format!("{session_id}.jsonl"));
    if !jsonl_path.exists() { return 0; }
    let Some(entries) = load_entries(&jsonl_path) else { return 0 };
    flush_entries(session_id, cwd, entries);
    let _ = std::fs::remove_file(&jsonl_path);
    try_ipc_refresh();
    0
}

/// Entry point for `pks flush-session <session_id> <cwd>`.
/// Always returns 0.
pub fn run_flush_session(session_id: &str, cwd: &Path) -> i32 {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let sessions_dir = PathBuf::from(home).join(".pks/sessions");
    flush_session_with_dir(session_id, cwd, &sessions_dir)
}

#[cfg(test)]
#[path = "flush_session_tests.rs"]
mod tests;
