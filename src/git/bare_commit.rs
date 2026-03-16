use std::path::PathBuf;
use git2::{Repository, Signature};

pub const PKS_BRANCH: &str = "pks-knowledge";

/// Writes files directly to the `pks-knowledge` git branch
/// using git2 plumbing — zero effect on the working tree or index.
pub struct BareCommit {
    repo_path: PathBuf,
}

impl BareCommit {
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self { repo_path: repo_path.into() }
    }

    /// Ensures the `pks-knowledge` branch exists.
    /// Creates it with an empty root commit if it doesn't.
    pub fn ensure_branch(&self) -> Result<(), git2::Error> {
        let repo = Repository::open(&self.repo_path)?;
        self.ensure_branch_on_repo(&repo)
    }

    /// Writes `content` at `file_path` within the `pks-knowledge` branch.
    /// Does NOT touch the working tree or the index.
    pub fn write_file(
        &self,
        file_path: &str,
        content: &[u8],
        message: &str,
    ) -> Result<git2::Oid, git2::Error> {
        let repo = Repository::open(&self.repo_path)?;
        self.ensure_branch_on_repo(&repo)?;
        let blob_oid = repo.blob(content)?;
        let branch_ref = repo.find_branch(PKS_BRANCH, git2::BranchType::Local)?;
        let parent_commit = branch_ref.get().peel_to_commit()?;
        let parent_tree = parent_commit.tree()?;
        let new_tree_oid = build_tree_with_file(&repo, Some(&parent_tree), file_path, blob_oid)?;
        let new_tree = repo.find_tree(new_tree_oid)?;
        let sig = bot_sig(&repo);
        repo.commit(
            Some(&format!("refs/heads/{PKS_BRANCH}")),
            &sig,
            &sig,
            message,
            &new_tree,
            &[&parent_commit],
        )
    }

    fn ensure_branch_on_repo(&self, repo: &Repository) -> Result<(), git2::Error> {
        if repo.find_branch(PKS_BRANCH, git2::BranchType::Local).is_ok() {
            return Ok(());
        }
        let tree_oid = repo.treebuilder(None)?.write()?;
        let tree = repo.find_tree(tree_oid)?;
        let sig = bot_sig(repo);
        repo.commit(
            Some(&format!("refs/heads/{PKS_BRANCH}")),
            &sig,
            &sig,
            "chore(pks): initialize pks-knowledge branch",
            &tree,
            &[],
        )?;
        Ok(())
    }
}

fn bot_sig(repo: &Repository) -> Signature<'static> {
    repo.signature()
        .unwrap_or_else(|_| Signature::now("pks-bot", "pks@localhost").unwrap())
}

/// Inserts `file_path` (possibly nested, e.g. `dir/sub/file.md`) with `blob_oid`
/// into a new tree based on `parent_tree`. Handles arbitrary depth recursively.
fn build_tree_with_file(
    repo: &Repository,
    parent_tree: Option<&git2::Tree>,
    file_path: &str,
    blob_oid: git2::Oid,
) -> Result<git2::Oid, git2::Error> {
    match file_path.split_once('/') {
        None => {
            let mut builder = repo.treebuilder(parent_tree)?;
            builder.insert(file_path, blob_oid, 0o100644)?;
            builder.write()
        }
        Some((dir_name, rest)) => {
            let existing_subtree = parent_tree
                .and_then(|t| t.get_name(dir_name))
                .and_then(|e| repo.find_tree(e.id()).ok());
            let new_subtree_oid = build_tree_with_file(
                repo,
                existing_subtree.as_ref(),
                rest,
                blob_oid,
            )?;
            let mut builder = repo.treebuilder(parent_tree)?;
            builder.insert(dir_name, new_subtree_oid, 0o040000)?;
            builder.write()
        }
    }
}

#[cfg(test)]
mod tests {
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
}
