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

    /// Writes multiple files in a SINGLE commit — opens the repo once.
    pub fn write_files_batch(
        &self,
        files: &[(&str, &[u8])],
        message: &str,
    ) -> Result<git2::Oid, git2::Error> {
        if files.is_empty() {
            return Err(git2::Error::from_str("empty file list"));
        }
        let repo = Repository::open(&self.repo_path)?;
        self.ensure_branch_on_repo(&repo)?;
        let branch_ref = repo.find_branch(PKS_BRANCH, git2::BranchType::Local)?;
        let parent_commit = branch_ref.get().peel_to_commit()?;
        let mut tree_oid = parent_commit.tree_id();
        for (file_path, content) in files {
            let blob_oid = repo.blob(content)?;
            let current_tree = repo.find_tree(tree_oid)?;
            tree_oid = build_tree_with_file(&repo, Some(&current_tree), file_path, blob_oid)?;
        }
        let new_tree = repo.find_tree(tree_oid)?;
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
#[path = "bare_commit_tests.rs"]
mod tests;
