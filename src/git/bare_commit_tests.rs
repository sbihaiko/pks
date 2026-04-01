use super::*;
use tempfile::TempDir;

fn init_bare_repo() -> TempDir {
    let tmp = tempfile::tempdir().unwrap();
    git2::Repository::init(tmp.path()).unwrap();
    let repo = Repository::open(tmp.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "pks-test").unwrap();
    config.set_str("user.email", "pks@test.local").unwrap();
    tmp
}

#[test]
fn ensure_branch_creates_pks_knowledge() {
    let tmp = init_bare_repo();
    let bc = BareCommit::new(tmp.path());
    bc.ensure_branch().unwrap();
    let repo = Repository::open(tmp.path()).unwrap();
    assert!(repo.find_branch(PKS_BRANCH, git2::BranchType::Local).is_ok());
}

#[test]
fn ensure_branch_is_idempotent() {
    let tmp = init_bare_repo();
    let bc = BareCommit::new(tmp.path());
    bc.ensure_branch().unwrap();
    bc.ensure_branch().unwrap();
}

#[test]
fn write_file_does_not_dirty_working_tree() {
    let tmp = init_bare_repo();
    let bc = BareCommit::new(tmp.path());
    bc.ensure_branch().unwrap();
    bc.write_file("test.md", b"hello pks", "test: add test.md").unwrap();
    let repo = Repository::open(tmp.path()).unwrap();
    let statuses = repo.statuses(None).unwrap();
    let working_tree_dirty: Vec<_> = statuses
        .iter()
        .filter(|s| !s.status().is_ignored())
        .collect();
    assert!(
        working_tree_dirty.is_empty(),
        "working tree must be clean after BareCommit"
    );
}

#[test]
fn write_files_batch_creates_single_commit() {
    let tmp = init_bare_repo();
    let bc = BareCommit::new(tmp.path());
    bc.ensure_branch().unwrap();
    let files: Vec<(&str, &[u8])> = vec![
        ("a.md", b"alpha"),
        ("dir/b.md", b"beta"),
        ("c.md", b"gamma"),
    ];
    bc.write_files_batch(&files, "batch: add 3 files").unwrap();
    let repo = Repository::open(tmp.path()).unwrap();
    let branch = repo.find_branch(PKS_BRANCH, git2::BranchType::Local).unwrap();
    let commit = branch.get().peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    assert!(tree.get_name("a.md").is_some());
    assert!(tree.get_name("c.md").is_some());
    let dir = repo.find_tree(tree.get_name("dir").unwrap().id()).unwrap();
    assert!(dir.get_name("b.md").is_some());
    // Exactly 2 commits: init + batch
    assert!(commit.parent(0).is_ok());
    assert!(commit.parent(0).unwrap().parent(0).is_err());
}

#[test]
fn write_file_content_readable_from_branch() {
    let tmp = init_bare_repo();
    let bc = BareCommit::new(tmp.path());
    bc.ensure_branch().unwrap();
    bc.write_file("note.md", b"pks content here", "test: add note").unwrap();
    let repo = Repository::open(tmp.path()).unwrap();
    let branch = repo
        .find_branch(PKS_BRANCH, git2::BranchType::Local)
        .unwrap();
    let commit = branch.get().peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    let entry = tree.get_name("note.md").unwrap();
    let blob = repo.find_blob(entry.id()).unwrap();
    assert_eq!(blob.content(), b"pks content here");
}
