use super::*;
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
fn is_initialized_false_before_init() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(!is_initialized(tmp.path()));
}

#[test]
fn generate_config_creates_valid_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join(".pks").join("config.toml");
    let git_common = PathBuf::from("/tmp/test.git");
    generate_config(&config_path, "my-project", &git_common).unwrap();
    let contents = std::fs::read_to_string(&config_path).unwrap();
    assert!(contents.contains("my-project"));
    assert!(contents.contains("git_common_dir"));
    assert!(contents.contains("provider = \"none\""));
}

#[test]
fn init_command_creates_config_and_branch() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());
    let cmd = InitCommand::new(tmp.path().to_path_buf(), false);
    cmd.run().expect("pks init must succeed");
    assert!(is_initialized(tmp.path()), ".pks/config.toml must exist");
    let repo = git2::Repository::open(tmp.path()).unwrap();
    assert!(
        repo.find_branch(crate::git::bare_commit::PKS_BRANCH, git2::BranchType::Local)
            .is_ok(),
        "pks-knowledge branch must be created"
    );
}

#[test]
fn init_command_is_idempotent_with_force() {
    let tmp = tempfile::tempdir().unwrap();
    init_repo_with_commit(tmp.path());
    InitCommand::new(tmp.path().to_path_buf(), false).run().unwrap();
    InitCommand::new(tmp.path().to_path_buf(), true).run().unwrap();
}

#[test]
fn init_command_errors_if_not_git_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let cmd = InitCommand::new(tmp.path().to_path_buf(), false);
    assert!(matches!(cmd.run(), Err(InitError::NotAGitRepo)));
}
