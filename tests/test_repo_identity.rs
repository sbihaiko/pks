//! Integration tests for M11 RepoIdentity.
//! Verifies that two linked worktrees of the same repository share the same RepoId.

use pks::git::RepoIdentity;
use std::path::Path;

/// Creates a git repo, adds an initial commit, and returns the temp dir.
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
fn two_worktrees_share_same_repo_id() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());

    let wt_path = tmp.path().join("feat");

    // git worktree add feat -b feat-branch
    let status = std::process::Command::new("git")
        .args([
            "-C",
            tmp.path().to_str().unwrap(),
            "worktree",
            "add",
            wt_path.to_str().unwrap(),
            "-b",
            "feat-branch",
        ])
        .status()
        .expect("git worktree add");
    assert!(status.success(), "git worktree add failed");

    let id_main = RepoIdentity::from_path(tmp.path())
        .expect("RepoIdentity from main worktree");
    let id_feat = RepoIdentity::from_path(&wt_path)
        .expect("RepoIdentity from linked worktree");

    assert_eq!(
        id_main.repo_id, id_feat.repo_id,
        "linked worktrees must share the same RepoId"
    );
    assert_eq!(id_main.git_common_dir, id_feat.git_common_dir);
}

#[test]
fn is_same_repo_true_for_linked_worktree() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());

    let wt_path = tmp.path().join("linked");
    let status = std::process::Command::new("git")
        .args([
            "-C",
            tmp.path().to_str().unwrap(),
            "worktree",
            "add",
            wt_path.to_str().unwrap(),
            "-b",
            "linked-branch",
        ])
        .status()
        .expect("git worktree add");
    assert!(status.success());

    assert!(
        RepoIdentity::is_same_repo(tmp.path(), &wt_path),
        "main and linked worktree must be identified as same repo"
    );
}

#[test]
fn is_same_repo_false_for_different_repos() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp1.path());
    init_repo_with_commit(tmp2.path());
    assert!(!RepoIdentity::is_same_repo(tmp1.path(), tmp2.path()));
}
