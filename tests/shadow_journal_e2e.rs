//! End-to-end integration test for M12 Shadow Journaling.
//! Simulates 5 tool events, flushes to pks-knowledge, and verifies the journal.

use pks::git::{BareCommit, bare_commit::PKS_BRANCH};
use pks::hooks::{ShadowJournalHook, ToolEvent};
use std::path::Path;

fn init_git_repo(dir: &Path) {
    let repo = git2::Repository::init(dir).expect("git init");
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "pks-test").unwrap();
    config.set_str("user.email", "pks@test.local").unwrap();
    drop(config);
    let sig = git2::Signature::now("pks-test", "pks@test.local").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "chore: init", &tree, &[]).unwrap();
}

fn make_event(tool: &str, summary: &str, path: Option<&str>, note: Option<&str>) -> ToolEvent {
    ToolEvent {
        tool_name: tool.to_string(),
        input_summary: summary.to_string(),
        outcome: "success".to_string(),
        file_paths: path.map(|p| vec![p.to_string()]).unwrap_or_default(),
        decision_note: note.map(|n| n.to_string()),
    }
}

#[test]
fn flush_five_events_creates_journal_in_branch() {
    let tmp = tempfile::tempdir().unwrap();
    init_git_repo(tmp.path());

    let bc = BareCommit::new(tmp.path());
    bc.ensure_branch().unwrap();

    let mut hook = ShadowJournalHook::new(tmp.path().to_path_buf(), "session-abc123".to_string());

    hook.record_tool_event(make_event("Edit", "added retry logic to src/retry.rs", Some("src/retry.rs"), Some("Moved retry logic into RetryPolicy struct")));
    hook.record_tool_event(make_event("Bash", "ran cargo test -- retry", None, None));
    hook.record_tool_event(make_event("Write", "created tests/retry_test.rs with e2e tests", Some("tests/retry_test.rs"), None));
    hook.record_tool_event(make_event("Edit", "updated Cargo.toml to add rand dependency", Some("Cargo.toml"), None));
    hook.record_tool_event(make_event("Bash", "ran cargo build to verify compilation", None, None));

    hook.flush_to_vault(&bc).expect("flush_to_vault must succeed");

    // Verify journal file exists in pks-knowledge branch
    let repo = git2::Repository::open(tmp.path()).unwrap();
    let branch = repo.find_branch(PKS_BRANCH, git2::BranchType::Local).unwrap();
    let commit = branch.get().peel_to_commit().unwrap();
    let root_tree = commit.tree().unwrap();

    // The journal path is journals/YYYY-MM-DD_session-abc123.md
    let journals_entry = root_tree.get_name("journals")
        .expect("journals/ directory must exist in pks-knowledge");
    let journals_tree = repo.find_tree(journals_entry.id()).unwrap();

    let mut found_journal = false;
    for entry in journals_tree.iter() {
        if let Some(name) = entry.name() {
            if name.contains("session-abc123") {
                found_journal = true;
                let blob = repo.find_blob(entry.id()).unwrap();
                let content = std::str::from_utf8(blob.content()).unwrap();
                assert!(content.contains("## Decisões"), "must have Decisões section");
                assert!(content.contains("## Arquivos Modificados"), "must have Arquivos section");
                assert!(content.contains("## Eventos Detalhados"), "must have Eventos section");
                assert!(content.contains("session-abc123"), "must contain session id");
                assert!(content.contains("RetryPolicy"), "must contain decision note");
                assert!(content.contains("src/retry.rs"), "must reference modified file");
            }
        }
    }
    assert!(found_journal, "journal file must exist in journals/ tree of pks-knowledge");
}

#[test]
fn flush_disabled_journal_writes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    init_git_repo(tmp.path());

    let bc = BareCommit::new(tmp.path());
    bc.ensure_branch().unwrap();

    let mut hook = ShadowJournalHook::new(tmp.path().to_path_buf(), "disabled-session".to_string());
    hook.config.enabled = false;
    hook.record_tool_event(make_event("Edit", "some edit", Some("file.rs"), None));
    let result = hook.flush_to_vault(&bc);

    assert!(result.is_ok(), "disabled flush must return Ok");
    // pks-knowledge branch exists but has empty root tree (only the init commit)
    let repo = git2::Repository::open(tmp.path()).unwrap();
    let branch = repo.find_branch(PKS_BRANCH, git2::BranchType::Local).unwrap();
    let commit = branch.get().peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    assert!(tree.get_name("journals").is_none(), "journals/ must not exist when disabled");
}
