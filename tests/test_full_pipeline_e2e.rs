//! Full pipeline end-to-end test for PKS.
//! Validates the entire chain: init → markdown → index → search → hook → journal → flush.
//! Covers PRD milestones M1-M8 integration criteria.

use std::sync::{Arc, Mutex};

use pks::boot_indexer::index_repo;
use pks::cli::init::InitCommand;
use pks::git::bare_commit::PKS_BRANCH;
use pks::git::BareCommit;
use pks::hooks::commit_event_log::{append_commit_event, flush_pending_commits};
use pks::hooks::{ShadowJournalHook, ToolEvent};
use pks::indexer::pipeline::IndexingPipeline;
use pks::search::retriever::SearchBackend;
use pks::state::PrevalentState;

fn init_repo_with_commit(dir: &std::path::Path) -> git2::Oid {
    let repo = git2::Repository::init(dir).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "pks-e2e").unwrap();
    cfg.set_str("user.email", "pks@e2e.local").unwrap();
    drop(cfg);
    let sig = git2::Signature::now("pks-e2e", "pks@e2e.local").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "feat: initial commit with project scaffold", &tree, &[]).unwrap()
}

#[tokio::test]
async fn full_pipeline_init_index_search_hook_journal() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_path = tmp.path();

    // === Phase 1: Init repo + pks init ===
    let initial_oid = init_repo_with_commit(repo_path);

    let cmd = InitCommand::new(repo_path.to_path_buf(), false);
    cmd.run().expect("pks init must succeed");

    // Verify pks-knowledge branch exists
    let repo = git2::Repository::open(repo_path).unwrap();
    assert!(
        repo.find_branch(PKS_BRANCH, git2::BranchType::Local).is_ok(),
        "pks-knowledge branch must exist after init"
    );
    drop(repo);

    // === Phase 2: Create markdown files ===
    let docs_dir = repo_path.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();

    std::fs::write(
        docs_dir.join("architecture.md"),
        "# Architecture\n\n## Overview\n\nThe system uses a prevalent daemon pattern.\n\n## Components\n\nBM25 search via Tantivy provides sub-millisecond keyword retrieval.\nThe double-buffered pipeline ensures queries never block during indexation.\n",
    ).unwrap();

    std::fs::write(
        docs_dir.join("decisions.md"),
        "# Decisions\n\n## ADR-001: Use Rust for the Daemon\n\nRust was chosen for zero GC pauses and memory safety.\nThe prevalent pattern requires predictable latency.\n\n## ADR-002: Git as Journal\n\nGit commit history serves as the append-only journal for durability.\n",
    ).unwrap();

    std::fs::write(
        repo_path.join("README.md"),
        "# Test Project\n\nThis project demonstrates the PKS pipeline end to end.\n",
    ).unwrap();

    // === Phase 3: Index via boot_indexer ===
    let state = Arc::new(Mutex::new(PrevalentState::default()));
    let mut pipeline = IndexingPipeline::new_from_env();

    index_repo(repo_path, &mut pipeline, &state).await;

    // Verify chunks were indexed
    {
        let mut guard = state.lock().unwrap();
        assert!(
            !guard.repos.is_empty(),
            "at least one repo must be registered after indexing"
        );
        guard.search_index.commit().unwrap();
    }

    // === Phase 4: BM25 Search ===
    {
        let mut guard = state.lock().unwrap();
        guard.search_index.commit().unwrap();

        let results = guard.search_index.search("prevalent daemon Tantivy", 10, None).unwrap();
        assert!(
            !results.is_empty(),
            "BM25 search for 'prevalent daemon Tantivy' must return results"
        );
        assert!(
            results.iter().any(|r| r.file_path.contains("architecture")),
            "architecture.md must appear in search results"
        );

        let results_adr = guard.search_index.search("Rust zero GC pauses", 10, None).unwrap();
        assert!(
            !results_adr.is_empty(),
            "BM25 search for ADR content must return results"
        );
        assert!(
            results_adr.iter().any(|r| r.file_path.contains("decisions")),
            "decisions.md must appear in ADR search results"
        );

        // Negative test: unrelated query should return no results
        let results_unrelated = guard.search_index.search("quantum physics entanglement", 5, None).unwrap();
        assert!(
            results_unrelated.is_empty(),
            "unrelated query should return no results from this corpus"
        );
    }

    // === Phase 5: Post-commit hook (append_commit_event) ===
    let sha = format!("{}", initial_oid);
    append_commit_event(repo_path, &sha, "main").expect("append_commit_event must succeed");

    let pending = repo_path.join(".git/pks_pending_commits.jsonl");
    assert!(pending.exists(), "pending commits JSONL must exist after hook");
    let pending_content = std::fs::read_to_string(&pending).unwrap();
    assert!(
        pending_content.contains(&sha[..7]),
        "pending commits must contain the commit SHA"
    );

    // === Phase 6: Flush pending commits (batch write to pks-knowledge) ===
    let flushed = flush_pending_commits(repo_path).expect("flush_pending_commits must succeed");
    assert!(flushed > 0, "at least 1 commit must be flushed");
    assert!(
        !pending.exists(),
        "pending commits file must be removed after flush"
    );

    // === Phase 7: Shadow journal (tool events → flush to vault) ===
    let bc = BareCommit::new(repo_path);

    let mut hook = ShadowJournalHook::new(repo_path.to_path_buf(), "e2e-session-001".to_string());

    hook.record_tool_event(ToolEvent {
        tool_name: "Edit".to_string(),
        input_summary: "refactored authentication module for clarity".to_string(),
        outcome: "success".to_string(),
        file_paths: vec!["src/auth.rs".to_string()],
        decision_note: Some("Separated JWT validation into its own function".to_string()),
    });
    hook.record_tool_event(ToolEvent {
        tool_name: "Bash".to_string(),
        input_summary: "ran cargo test to verify all tests pass".to_string(),
        outcome: "success".to_string(),
        file_paths: vec![],
        decision_note: None,
    });
    hook.record_tool_event(ToolEvent {
        tool_name: "Write".to_string(),
        input_summary: "created integration test for auth flow".to_string(),
        outcome: "success".to_string(),
        file_paths: vec!["tests/auth_test.rs".to_string()],
        decision_note: None,
    });

    hook.flush_to_vault(&bc).expect("flush_to_vault must succeed");

    // === Phase 8: Verify journal on pks-knowledge branch (via git2, no checkout) ===
    let repo = git2::Repository::open(repo_path).unwrap();
    assert_journal_in_vault(&repo, "e2e-session-001");
}

fn assert_journal_in_vault(repo: &git2::Repository, session_id: &str) {
    let branch = repo.find_branch(PKS_BRANCH, git2::BranchType::Local).unwrap();
    let commit = branch.get().peel_to_commit().unwrap();
    let root_tree = commit.tree().unwrap();

    let journals_entry = root_tree.get_name("journals")
        .expect("journals/ directory must exist in pks-knowledge");
    let journals_tree = repo.find_tree(journals_entry.id()).unwrap();

    let journal_entry = journals_tree.iter()
        .find(|e| e.name().is_some_and(|n| n.contains(session_id)))
        .unwrap_or_else(|| panic!("journal for {session_id} must exist in pks-knowledge"));

    let blob = repo.find_blob(journal_entry.id()).unwrap();
    let content = std::str::from_utf8(blob.content()).unwrap();
    assert!(content.contains("Decisões"), "journal must have Decisões section");
    assert!(content.contains("JWT validation"), "journal must contain decision note");
    assert!(content.contains("src/auth.rs"), "journal must reference modified files");
    assert!(content.contains("Eventos Detalhados"), "journal must have events table");
}
