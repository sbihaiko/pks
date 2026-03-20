use pks::git_journal_append::{append_commit_to_daily_log, JournalConfig};
use pks::git_journal_date::current_date_utc;
use std::path::Path;
use git2::Repository;
use pks::git::bare_commit::PKS_BRANCH;

fn make_config(vault_root: &str) -> JournalConfig {
    JournalConfig {
        vault_root: vault_root.to_string(),
        enabled: true,
        allow_prefixes: vec!["feat".to_string(), "fix".to_string(), "docs".to_string()],
        min_words: 3,
        ignore_authors: vec![],
    }
}

fn create_test_repo_with_commit(dir: &Path, subject: &str) -> String {
    let repo = git2::Repository::init(dir).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "Test Author").unwrap();
    cfg.set_str("user.email", "test@example.com").unwrap();

    let sig = git2::Signature::now("Test Author", "test@example.com").unwrap();
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_id).unwrap();
    let oid = repo.commit(Some("HEAD"), &sig, &sig, subject, &tree, &[]).unwrap();
    oid.to_string()
}

fn read_log_from_branch(dir: &Path, filename: &str) -> String {
    let repo = Repository::open(dir).unwrap();
    let branch = repo.find_branch(PKS_BRANCH, git2::BranchType::Local).unwrap();
    let commit = branch.get().peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    let entry = tree.get_name(filename).expect("file not found in branch");
    let blob = repo.find_blob(entry.id()).unwrap();
    std::str::from_utf8(blob.content()).unwrap().to_owned()
}

fn journal_exists_in_branch(dir: &Path, filename: &str) -> bool {
    let repo = match Repository::open(dir) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let branch = match repo.find_branch(PKS_BRANCH, git2::BranchType::Local) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let commit = branch.get().peel_to_commit().unwrap_or_else(|_| panic!("failed to peel commit"));
    let tree = commit.tree().unwrap();
    let exists = tree.get_name(filename).is_some();
    exists
}

fn log_filename(vault: &str) -> String {
    format!("{}_journal_{}.md", vault, current_date_utc())
}

#[test]
fn journal_e2e_creates_log_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sha = create_test_repo_with_commit(tmp.path(), "feat: add user authentication flow");
    let config = make_config("vault");

    append_commit_to_daily_log(tmp.path(), &sha, "main", &config).unwrap();

    let filename = log_filename("vault");
    assert!(journal_exists_in_branch(tmp.path(), &filename), "log file must exist in branch");
    let contents = read_log_from_branch(tmp.path(), &filename);
    assert!(contents.contains(&sha[..7]), "log must contain short sha");
    assert!(contents.contains("feat: add user authentication flow"), "log must contain subject");
}

#[test]
fn journal_e2e_two_commits_append_in_order() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sha1 = create_test_repo_with_commit(tmp.path(), "feat: add login page for users");

    let repo = git2::Repository::open(tmp.path()).unwrap();
    let sig = git2::Signature::now("Test Author", "test@example.com").unwrap();
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.find_commit(git2::Oid::from_str(&sha1).unwrap()).unwrap();
    let sha2 = repo
        .commit(Some("HEAD"), &sig, &sig, "fix: correct redirect after login", &tree, &[&parent])
        .unwrap()
        .to_string();

    let config = make_config("vault");
    append_commit_to_daily_log(tmp.path(), &sha1, "main", &config).unwrap();
    append_commit_to_daily_log(tmp.path(), &sha2, "main", &config).unwrap();

    let contents = read_log_from_branch(tmp.path(), &log_filename("vault"));
    assert!(contents.contains(&sha1[..7]), "log must contain first sha");
    assert!(contents.contains(&sha2[..7]), "log must contain second sha");
    assert_eq!(contents.lines().count(), 2, "log must have exactly 2 lines");
}

#[test]
fn journal_e2e_skips_pks_knowledge_branch() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sha = create_test_repo_with_commit(tmp.path(), "feat: add syncing for knowledge vault");
    let config = make_config("vault");

    append_commit_to_daily_log(tmp.path(), &sha, "pks-knowledge", &config).unwrap();

    assert!(!journal_exists_in_branch(tmp.path(), &log_filename("vault")), "log file must NOT be created in branch for pks-knowledge branch");
}

#[test]
fn journal_e2e_skips_non_conventional_subject() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sha = create_test_repo_with_commit(tmp.path(), "wip: fix stuff and things here");
    let config = make_config("vault");

    append_commit_to_daily_log(tmp.path(), &sha, "main", &config).unwrap();

    assert!(!journal_exists_in_branch(tmp.path(), &log_filename("vault")), "log file must NOT be created in branch for non-conventional subject");
}

#[test]
fn journal_fswatcher_smoke_log_file_is_valid_markdown() {
    let tmp = tempfile::TempDir::new().unwrap();
    let sha = create_test_repo_with_commit(tmp.path(), "docs: update readme with setup guide");
    let config = make_config("vault");

    append_commit_to_daily_log(tmp.path(), &sha, "main", &config).unwrap();

    let filename = log_filename("vault");
    let contents = read_log_from_branch(tmp.path(), &filename);
    let line = contents.lines().next().unwrap_or("");
    assert!(line.starts_with("- **"), "markdown line must start with '- **'");
    assert!(line.contains('`'), "markdown line must contain backtick-quoted sha");
    assert!(line.contains(": "), "markdown line must contain ': ' separator");
}
