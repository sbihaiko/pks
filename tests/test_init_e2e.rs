//! End-to-end integration test for M14 pks init.
//! Verifies all onboarding steps on a real Git repository.

use pks::cli::init::{InitCommand, is_initialized};
use pks::git::bare_commit::PKS_BRANCH;
use std::path::Path;

fn init_repo_with_commit(dir: &Path) {
    let repo = git2::Repository::init(dir).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "pks-test").unwrap();
    cfg.set_str("user.email", "pks@test.local").unwrap();
    drop(cfg);
    let sig = git2::Signature::now("pks-test", "pks@test.local").unwrap();
    let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[]).unwrap();
}

#[test]
fn pks_init_creates_all_expected_artifacts() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());

    assert!(!is_initialized(tmp.path()), "must not be initialized before pks init");

    let cmd = InitCommand::new(tmp.path().to_path_buf(), false);
    cmd.run().expect("pks init must succeed");

    // .pks/config.toml must exist with correct content
    let config_path = tmp.path().join(".pks").join("config.toml");
    assert!(config_path.exists(), ".pks/config.toml must be created");
    let config_contents = std::fs::read_to_string(&config_path).unwrap();
    assert!(config_contents.contains("git_common_dir"), "config must contain git_common_dir");
    assert!(config_contents.contains("provider = \"none\""), "config must have embedding provider none");

    // pks-knowledge branch must be created
    let repo = git2::Repository::open(tmp.path()).unwrap();
    assert!(
        repo.find_branch(PKS_BRANCH, git2::BranchType::Local).is_ok(),
        "pks-knowledge branch must be created"
    );

    // is_initialized must return true
    assert!(is_initialized(tmp.path()), "is_initialized must be true after pks init");

    // .git/info/exclude must mention .pks/
    let exclude = tmp.path().join(".git").join("info").join("exclude");
    let exclude_contents = std::fs::read_to_string(exclude).unwrap_or_default();
    assert!(exclude_contents.contains(".pks/"), ".git/info/exclude must contain .pks/");
}

#[test]
fn pks_init_second_run_without_force_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());
    InitCommand::new(tmp.path().to_path_buf(), false).run().unwrap();
    let result = InitCommand::new(tmp.path().to_path_buf(), false).run();
    assert!(result.is_err(), "second init without --force must return error");
}

#[test]
fn pks_init_force_overwrites_config() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());
    InitCommand::new(tmp.path().to_path_buf(), false).run().unwrap();
    // Overwrite config manually with wrong content
    let config_path = tmp.path().join(".pks").join("config.toml");
    std::fs::write(&config_path, "corrupted").unwrap();
    InitCommand::new(tmp.path().to_path_buf(), true).run().unwrap();
    let new_contents = std::fs::read_to_string(&config_path).unwrap();
    assert!(new_contents.contains("git_common_dir"), "config must be regenerated with --force");
}

#[test]
fn pks_init_not_git_repo_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let result = InitCommand::new(tmp.path().to_path_buf(), false).run();
    assert!(result.is_err(), "init in non-git dir must fail");
}
