use super::*;
use std::fs;
use tempfile::TempDir;

fn make_fake_git_dir(tmp: &TempDir) {
    fs::create_dir_all(tmp.path().join(".git")).unwrap();
}

#[test]
fn append_commit_event_writes_valid_jsonl() {
    let tmp = TempDir::new().unwrap();
    make_fake_git_dir(&tmp);
    append_commit_event(tmp.path(), "abc1234", "main").unwrap();
    let path = tmp.path().join(".git").join(PENDING_COMMITS_FILE);
    let content = fs::read_to_string(&path).unwrap();
    let event: CommitEvent = serde_json::from_str(content.trim()).unwrap();
    assert_eq!(event.sha, "abc1234");
    assert_eq!(event.branch, "main");
}

#[test]
fn append_commit_event_skips_pks_knowledge_branch() {
    let tmp = TempDir::new().unwrap();
    make_fake_git_dir(&tmp);
    append_commit_event(tmp.path(), "abc1234", "pks-knowledge").unwrap();
    let path = tmp.path().join(".git").join(PENDING_COMMITS_FILE);
    assert!(!path.exists());
}

#[test]
fn flush_pending_commits_returns_zero_when_no_file() {
    let tmp = TempDir::new().unwrap();
    make_fake_git_dir(&tmp);
    let result = flush_pending_commits(tmp.path()).unwrap();
    assert_eq!(result, 0);
}

#[test]
fn flush_pending_commits_atomic_rename() {
    let tmp = TempDir::new().unwrap();
    make_fake_git_dir(&tmp);
    let pending = tmp.path().join(".git").join(PENDING_COMMITS_FILE);
    fs::write(&pending, "").unwrap();
    // flush with empty content — should return 0 and clean up
    let result = flush_pending_commits(tmp.path()).unwrap();
    assert_eq!(result, 0);
    assert!(!pending.exists());
    assert!(!tmp.path().join(".git").join(PROCESSING_FILE).exists());
}
