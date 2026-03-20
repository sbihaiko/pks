use super::*;
use chrono::Utc;
use git2::Repository;
use std::path::Path;
use tempfile::TempDir;

fn make_entry(summary: &str) -> JournalEntry {
    JournalEntry {
        timestamp: Utc::now(),
        tool_name: "Edit".to_string(),
        tool_input_summary: summary.to_string(),
        outcome: "success".to_string(),
        file_paths: vec!["src/main.rs".to_string()],
        decision_note: None,
    }
}

fn write_jsonl(dir: &Path, session_id: &str, entries: &[JournalEntry]) {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{session_id}.jsonl"));
    let lines: Vec<String> = entries
        .iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect();
    std::fs::write(path, lines.join("\n")).unwrap();
}

fn init_repo() -> TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let repo = Repository::init(tmp.path()).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "pks-test").unwrap();
    cfg.set_str("user.email", "pks@test.local").unwrap();
    tmp
}

#[test]
fn nonexistent_jsonl_returns_0() {
    let sessions_dir = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let result = flush_session_with_dir("missing-session", cwd.path(), sessions_dir.path());
    assert_eq!(result, 0);
}

#[test]
fn min_words_guard_deletes_file_without_commit() {
    let sessions_dir = tempfile::tempdir().unwrap();
    let repo_dir = init_repo();

    // Only 2 words — below default threshold of 10
    let entries = vec![make_entry("two words")];
    write_jsonl(sessions_dir.path(), "session-001", &entries);

    let jsonl_path = sessions_dir.path().join("session-001.jsonl");
    assert!(jsonl_path.exists());

    let result = flush_session_with_dir("session-001", repo_dir.path(), sessions_dir.path());
    assert_eq!(result, 0);
    assert!(!jsonl_path.exists(), "JSONL must be deleted after min_words guard");

    // No commit should have been made
    let repo = Repository::open(repo_dir.path()).unwrap();
    assert!(
        repo.find_branch("pks-knowledge", git2::BranchType::Local).is_err(),
        "pks-knowledge must not exist when min_words guard fires"
    );
}

fn find_journal_content(repo_path: &Path, session_id: &str) -> Option<String> {
    let repo = Repository::open(repo_path).unwrap();
    let branch = repo.find_branch("pks-knowledge", git2::BranchType::Local).ok()?;
    let tree = branch.get().peel_to_commit().unwrap().tree().unwrap();
    let jt = repo.find_tree(tree.get_name("journals")?.id()).unwrap();
    for i in 0..jt.len() {
        let entry = jt.get(i).unwrap();
        if entry.name().unwrap_or("").contains(session_id) {
            let blob = repo.find_blob(entry.id()).unwrap();
            return Some(std::str::from_utf8(blob.content()).unwrap().to_string());
        }
    }
    None
}

#[test]
fn full_flow_commits_journal_and_deletes_jsonl() {
    let sessions_dir = tempfile::tempdir().unwrap();
    let repo_dir = init_repo();
    let entries = vec![
        make_entry("refactored the authentication module to use JWT tokens and updated tests"),
        make_entry("added integration tests for the login flow endpoint"),
    ];
    write_jsonl(sessions_dir.path(), "session-abc", &entries);
    let jsonl_path = sessions_dir.path().join("session-abc.jsonl");

    let result = flush_session_with_dir("session-abc", repo_dir.path(), sessions_dir.path());
    assert_eq!(result, 0);
    assert!(!jsonl_path.exists(), "JSONL must be deleted after successful flush");

    let content = find_journal_content(repo_dir.path(), "session-abc")
        .expect("journal file for session-abc must exist");
    assert!(content.contains("session-abc"));
    assert!(content.contains("refactored"));
}
