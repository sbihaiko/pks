use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::git_journal_append::{
    JournalConfig, branch_log_filename, format_log_line, is_pks_knowledge_branch,
    is_ignored_author, passes_conventional_prefix, passes_min_words, read_commit_metadata,
};
use crate::git_journal_date::unix_timestamp_to_date;

pub const PENDING_COMMITS_FILE: &str = "pks_pending_commits.jsonl";
pub const PROCESSING_FILE: &str = "pks_pending_commits.processing.jsonl";

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitEvent {
    pub sha: String,
    pub branch: String,
    pub repo: String,
    pub ts: i64,
}

pub fn append_commit_event(
    repo_root: &Path,
    sha: &str,
    branch: &str,
) -> Result<(), std::io::Error> {
    if is_pks_knowledge_branch(branch) {
        return Ok(());
    }
    let event = CommitEvent {
        sha: sha.to_string(),
        branch: branch.to_string(),
        repo: repo_root.to_string_lossy().to_string(),
        ts: chrono::Utc::now().timestamp(),
    };
    let line = serde_json::to_string(&event)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let path = repo_root.join(".git").join(PENDING_COMMITS_FILE);
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn flush_pending_commits(repo_root: &Path) -> Result<usize, String> {
    let pending = repo_root.join(".git").join(PENDING_COMMITS_FILE);
    let processing = repo_root.join(".git").join(PROCESSING_FILE);
    let events = atomically_load_events(&pending, &processing)?;
    if events.is_empty() {
        return Ok(0);
    }
    let count = events.len();
    if let Err(e) = write_events_to_branch(repo_root, &events) {
        let _ = fs::rename(&processing, &pending);
        return Err(e);
    }
    let _ = fs::remove_file(&processing);
    Ok(count)
}

fn atomically_load_events(pending: &Path, processing: &Path) -> Result<Vec<CommitEvent>, String> {
    let mut content = recover_stale_processing(processing);
    if pending.exists() {
        fs::rename(pending, processing).map_err(|e| format!("rename failed: {e}"))?;
        let new = fs::read_to_string(processing)
            .map_err(|e| format!("read processing file failed: {e}"))?;
        content.push_str(&new);
    }
    if content.is_empty() {
        let _ = fs::remove_file(processing);
        return Ok(Vec::new());
    }
    fs::write(processing, &content).map_err(|e| format!("write merged: {e}"))?;
    Ok(parse_events(&content))
}

fn recover_stale_processing(processing: &Path) -> String {
    if processing.exists() {
        fs::read_to_string(processing).unwrap_or_default()
    } else {
        String::new()
    }
}

fn parse_events(content: &str) -> Vec<CommitEvent> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn write_events_to_branch(repo_root: &Path, events: &[CommitEvent]) -> Result<(), String> {
    use crate::git::BareCommit;
    let config = JournalConfig::from_env();
    let bc = BareCommit::new(repo_root);
    bc.ensure_branch().map_err(|e| format!("ensure_branch: {e}"))?;
    let by_date = group_events_by_date(repo_root, &config, events);
    commit_grouped_lines(&bc, repo_root, &config.vault_root, &by_date)
}

fn group_events_by_date(
    repo_root: &Path,
    config: &JournalConfig,
    events: &[CommitEvent],
) -> std::collections::HashMap<String, String> {
    let mut by_date: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for event in events {
        if let Some(line) = format_filtered_event(repo_root, config, event) {
            let date = unix_timestamp_to_date(event.ts);
            by_date.entry(date).or_default().push_str(&line);
        }
    }
    by_date
}

fn format_filtered_event(repo_root: &Path, config: &JournalConfig, event: &CommitEvent) -> Option<String> {
    if is_pks_knowledge_branch(&event.branch) {
        return None;
    }
    let meta = read_commit_metadata(repo_root, &event.sha).ok()?;
    if is_ignored_author(&meta.author, &config.ignore_authors) {
        return None;
    }
    if !passes_conventional_prefix(&meta.subject, &config.allow_prefixes) {
        return None;
    }
    if !passes_min_words(&meta.subject, config.min_words) {
        return None;
    }
    Some(format_log_line(&meta))
}

fn commit_grouped_lines(
    bc: &crate::git::BareCommit,
    repo_root: &Path,
    vault_root: &str,
    by_date: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    use crate::git_journal_append::read_log_from_branch_pub;
    let mut file_contents: Vec<(String, Vec<u8>)> = Vec::new();
    for (date, lines) in by_date {
        let filename = branch_log_filename(vault_root, date);
        let existing = read_log_from_branch_pub(repo_root, &filename).unwrap_or_default();
        let new_content = format!("{existing}{lines}");
        file_contents.push((filename, new_content.into_bytes()));
    }
    let files: Vec<(&str, &[u8])> = file_contents
        .iter()
        .map(|(name, content)| (name.as_str(), content.as_slice()))
        .collect();
    let dates: Vec<&String> = by_date.keys().collect();
    let msg = if dates.len() == 1 {
        format!("pks(journal): batch append commits to {}", dates[0])
    } else {
        let mut sorted = dates.clone();
        sorted.sort();
        format!("pks(journal): batch append commits to {}..{}", sorted[0], sorted[sorted.len() - 1])
    };
    bc.write_files_batch(&files, &msg)
        .map_err(|e| format!("write_files_batch: {e}"))?;
    Ok(())
}

#[cfg(test)]
#[path = "commit_event_log_tests.rs"]
mod tests;
