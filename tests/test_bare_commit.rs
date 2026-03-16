//! Integration tests for M11 BareCommit.
//! Verifies that writing to pks-knowledge branch does not modify the working tree.

use pks::git::BareCommit;
use std::path::Path;

fn init_repo_with_commit(dir: &Path) {
    let repo = git2::Repository::init(dir).expect("git init");
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "pks-test").unwrap();
    config.set_str("user.email", "pks@test.local").unwrap();
    drop(config);

    let sig = git2::Signature::now("pks-test", "pks@test.local").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "chore: init", &tree, &[])
        .unwrap();
}

#[test]
fn bare_commit_does_not_dirty_working_tree() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());

    let bc = BareCommit::new(tmp.path());
    bc.ensure_branch().unwrap();
    bc.write_file("test.md", b"pks content", "test: add test.md").unwrap();

    let repo = git2::Repository::open(tmp.path()).unwrap();
    let statuses = repo.statuses(None).unwrap();
    let dirty: Vec<_> = statuses
        .iter()
        .filter(|s| !s.status().is_ignored())
        .collect();
    assert!(dirty.is_empty(), "working tree must be clean after BareCommit");
}

#[test]
fn bare_commit_content_readable_from_branch() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());

    let bc = BareCommit::new(tmp.path());
    bc.ensure_branch().unwrap();
    bc.write_file("journal.md", b"entry one\n", "test: add journal").unwrap();

    let repo = git2::Repository::open(tmp.path()).unwrap();
    let branch = repo
        .find_branch(pks::git::bare_commit::PKS_BRANCH, git2::BranchType::Local)
        .unwrap();
    let commit = branch.get().peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    let entry = tree.get_name("journal.md").unwrap();
    let blob = repo.find_blob(entry.id()).unwrap();
    assert_eq!(blob.content(), b"entry one\n");
}

#[test]
fn bare_commit_with_linked_worktree_does_not_error() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());

    let wt_path = tmp.path().join("wt");
    let status = std::process::Command::new("git")
        .args([
            "-C",
            tmp.path().to_str().unwrap(),
            "worktree",
            "add",
            wt_path.to_str().unwrap(),
            "-b",
            "wt-branch",
        ])
        .status()
        .expect("git worktree add");
    assert!(status.success());

    // Writing from linked worktree must succeed
    let bc = BareCommit::new(&wt_path);
    bc.ensure_branch().unwrap();
    bc.write_file("from-wt.md", b"written from linked wt", "test: from wt")
        .unwrap();

    // And the working tree of the main repo must still be clean
    let repo = git2::Repository::open(tmp.path()).unwrap();
    let statuses = repo.statuses(None).unwrap();
    let dirty: Vec<_> = statuses
        .iter()
        .filter(|s| !s.status().is_ignored())
        .collect();
    assert!(dirty.is_empty(), "main working tree must be clean");
}
