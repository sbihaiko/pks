use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub enum BranchError {
    GitCommand(String),
    Io(std::io::Error),
    WorktreeAlreadyExists,
}

impl From<std::io::Error> for BranchError {
    fn from(e: std::io::Error) -> Self { BranchError::Io(e) }
}

impl std::fmt::Display for BranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchError::GitCommand(s) => write!(f, "git command failed: {s}"),
            BranchError::Io(e) => write!(f, "io error: {e}"),
            BranchError::WorktreeAlreadyExists => write!(f, "worktree already exists"),
        }
    }
}

pub const PKS_BRANCH: &str = "pks-knowledge";
pub const PROMETHEUS_DIR: &str = "prometheus";

pub fn branch_exists(repo_root: &Path) -> bool {
    run_git(repo_root, &["rev-parse", "--verify", PKS_BRANCH]).is_ok()
}

pub fn worktree_exists(repo_root: &Path) -> bool {
    let prometheus = repo_root.join(PROMETHEUS_DIR);
    prometheus.is_dir() && prometheus.join(".git").exists()
}

pub fn create_pks_branch_and_worktree(repo_root: &Path) -> Result<(), BranchError> {
    if branch_exists(repo_root) && worktree_exists(repo_root) {
        return Ok(());
    }

    if !branch_exists(repo_root) {
        run_git(repo_root, &["checkout", "--orphan", PKS_BRANCH])?;
        run_git(repo_root, &["rm", "-rf", "--cached", "."])?;
        run_git(repo_root, &[
            "commit",
            "--allow-empty",
            "-m",
            "chore(pks): initialize pks-knowledge orphan branch",
        ])?;
        let main = detect_main_branch(repo_root);
        run_git(repo_root, &["checkout", &main])?;
    }

    if !worktree_exists(repo_root) {
        let prometheus_path = repo_root.join(PROMETHEUS_DIR);
        let prometheus_str = prometheus_path.to_string_lossy();
        run_git(repo_root, &["worktree", "add", &prometheus_str, PKS_BRANCH])?;
    }

    Ok(())
}

pub fn remove_worktree(repo_root: &Path) -> Result<(), BranchError> {
    let prometheus_path = repo_root.join(PROMETHEUS_DIR);
    let prometheus_str = prometheus_path.to_string_lossy();
    run_git(repo_root, &["worktree", "remove", "--force", &prometheus_str])?;
    Ok(())
}

pub fn commit_to_pks_knowledge(
    prometheus_root: &Path,
    file_rel: &str,
    content: &str,
    source_commit_sha: &str,
    message: &str,
) -> Result<(), BranchError> {
    let file_path = prometheus_root.join(file_rel);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let final_content = if content.starts_with("---") {
        content.replacen(
            "---\n",
            &format!("---\nsource_commit_sha: {source_commit_sha}\n"),
            1,
        )
    } else {
        format!("---\nsource_commit_sha: {source_commit_sha}\n---\n\n{content}")
    };

    std::fs::write(&file_path, final_content)?;

    run_git(prometheus_root, &["add", file_rel])?;
    run_git(prometheus_root, &["commit", "-m", message])?;
    Ok(())
}

fn detect_main_branch(repo_root: &Path) -> String {
    if run_git(repo_root, &["rev-parse", "--verify", "main"]).is_ok() {
        return "main".to_string();
    }
    "master".to_string()
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, BranchError> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(BranchError::Io)?;

    if !output.status.success() {
        return Err(BranchError::GitCommand(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_head_sha(repo_root: &Path) -> Option<String> {
    run_git(repo_root, &["rev-parse", "HEAD"]).ok()
}

pub fn get_current_branch(repo_root: &Path) -> Option<String> {
    run_git(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_exists_returns_false_for_nonexistent() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(!branch_exists(tmp.path()));
    }

    #[test]
    fn worktree_exists_returns_false_when_no_prometheus_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(!worktree_exists(tmp.path()));
    }

    #[test]
    fn get_head_sha_returns_none_for_non_git() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(get_head_sha(tmp.path()).is_none());
    }
}
