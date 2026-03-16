use std::path::{Path, PathBuf};

/// Stable identifier for a Git repository.
/// Derived from the canonical path of `git-common-dir`, which is
/// invariant to which worktree is active.
pub type RepoId = String;

/// Identifies a Git repository by its canonical git-common-dir path.
///
/// Invariant: two worktrees of the same repo produce equal `repo_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoIdentity {
    /// Canonical path to the shared git directory (invariant across worktrees).
    /// Used as the stable primary key for this repository in PrevalentState.
    pub repo_id: RepoId,
    /// Raw PathBuf of the git-common-dir (before string conversion).
    pub git_common_dir: PathBuf,
}

/// Reads the `commondir` pointer file and resolves the path, if present.
fn read_commondir_pointer(git_dir: &Path) -> Option<PathBuf> {
    let file = git_dir.join("commondir");
    let contents = std::fs::read_to_string(&file).ok()?;
    let trimmed = contents.trim();
    let candidate = if Path::new(trimmed).is_absolute() {
        PathBuf::from(trimmed)
    } else {
        git_dir.join(trimmed)
    };
    candidate.exists().then_some(candidate)
}

/// Resolves the git-common-dir from a git directory path.
///
/// For linked worktrees the git dir contains a `commondir` file pointing
/// to the main repo's git dir. For normal repos the git dir is its own common dir.
fn resolve_common_dir(git_dir: &Path) -> PathBuf {
    let target = read_commondir_pointer(git_dir).unwrap_or_else(|| git_dir.to_path_buf());
    std::fs::canonicalize(&target).unwrap_or(target)
}

impl RepoIdentity {
    /// Opens the repository at `path` (or any ancestor) and returns its identity.
    ///
    /// Returns `Err` if `path` is not inside a Git repository.
    pub fn from_path(path: &Path) -> Result<Self, git2::Error> {
        let repo = git2::Repository::discover(path)?;
        let canonical = resolve_common_dir(repo.path());
        Ok(Self {
            repo_id: canonical.to_string_lossy().into_owned(),
            git_common_dir: canonical,
        })
    }

    /// Returns true if two paths belong to the same Git repository.
    pub fn is_same_repo(a: &Path, b: &Path) -> bool {
        match (Self::from_path(a), Self::from_path(b)) {
            (Ok(ra), Ok(rb)) => ra.git_common_dir == rb.git_common_dir,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn init_git_repo(dir: &Path) {
        git2::Repository::init(dir).expect("init repo");
    }

    #[test]
    fn from_path_returns_err_on_non_git_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(RepoIdentity::from_path(tmp.path()).is_err());
    }

    #[test]
    fn from_path_returns_ok_on_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());
        let identity = RepoIdentity::from_path(tmp.path()).unwrap();
        assert!(!identity.repo_id.is_empty());
        assert!(identity.git_common_dir.exists());
    }

    #[test]
    fn is_same_repo_true_for_same_path() {
        let tmp = tempfile::tempdir().unwrap();
        init_git_repo(tmp.path());
        assert!(RepoIdentity::is_same_repo(tmp.path(), tmp.path()));
    }

    #[test]
    fn is_same_repo_false_for_different_repos() {
        let tmp1 = tempfile::tempdir().unwrap();
        let tmp2 = tempfile::tempdir().unwrap();
        init_git_repo(tmp1.path());
        init_git_repo(tmp2.path());
        assert!(!RepoIdentity::is_same_repo(tmp1.path(), tmp2.path()));
    }

    #[test]
    fn is_same_repo_false_for_non_git_dir() {
        let tmp1 = tempfile::tempdir().unwrap();
        let tmp2 = tempfile::tempdir().unwrap();
        init_git_repo(tmp1.path());
        assert!(!RepoIdentity::is_same_repo(tmp1.path(), tmp2.path()));
    }
}
