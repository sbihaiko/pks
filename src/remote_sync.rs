use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::git_journal::CommitInfo;

fn poll_interval() -> Duration {
    let secs = std::env::var("PKS_REMOTE_POLL_INTERVAL")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(300);
    Duration::from_secs(secs)
}

pub struct RemoteSync {
    last_fetched: HashMap<PathBuf, Instant>,
    poll_interval: Duration,
    commit_tx: mpsc::SyncSender<CommitInfo>,
}

impl RemoteSync {
    pub fn new(commit_tx: mpsc::SyncSender<CommitInfo>) -> Self {
        Self {
            last_fetched: HashMap::new(),
            poll_interval: poll_interval(),
            commit_tx,
        }
    }

    pub fn poll_all(&mut self, vaults_dir: &Path) {
        let Ok(entries) = std::fs::read_dir(vaults_dir) else {
            return;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() || !path.join(".git").is_dir() {
                continue;
            }
            if self.needs_fetch(&path) {
                self.fetch_repo(&path);
            }
        }
    }

    fn needs_fetch(&self, repo_path: &Path) -> bool {
        match self.last_fetched.get(repo_path) {
            None => true,
            Some(&last) => last.elapsed() >= self.poll_interval,
        }
    }

    fn fetch_repo(&mut self, repo_path: &Path) {
        let repo_id = repo_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| repo_path.to_string_lossy().into_owned());

        let pre_sha = get_head_sha(repo_path);
        let result = git_fetch(repo_path);
        self.last_fetched.insert(repo_path.to_path_buf(), Instant::now());

        match result {
            Ok(_) => {
                info!(repo_id = %repo_id, "git fetch succeeded");
                let post_sha = get_head_sha(repo_path);
                if pre_sha != post_sha {
                    if let Some(sha) = post_sha {
                        let branch = get_current_branch(repo_path).unwrap_or_else(|| "main".to_string());
                        let info = CommitInfo {
                            repo_id,
                            repo_path: repo_path.to_path_buf(),
                            branch,
                            commit_sha: sha,
                            tree_hash: None,
                        };
                        let _ = self.commit_tx.try_send(info);
                    }
                }
            }
            Err(e) => {
                warn!(repo_id = %repo_id, error = %e, "git fetch failed — partition isolated");
            }
        }
    }
}

fn make_clone_url(url: &str) -> String {
    let Ok(token) = std::env::var("GITHUB_TOKEN") else {
        return url.to_string();
    };
    if !url.starts_with("https://github.com") {
        return url.to_string();
    }
    url.replace("https://github.com", &format!("https://{token}@github.com"))
}

pub fn clone_remote_repo(url: &str, vaults_dir: &Path, name: &str) -> Result<PathBuf, String> {
    let dest = vaults_dir.join(name);
    if dest.exists() {
        return Ok(dest);
    }
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(vaults_dir).arg("clone").arg(&make_clone_url(url)).arg(name);
    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(dest)
}

fn git_fetch(repo_path: &Path) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(["fetch", "--quiet", "--all"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    Ok(())
}

fn get_head_sha(repo_path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_current_branch(repo_path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_interval_defaults_to_300s() {
        std::env::remove_var("PKS_REMOTE_POLL_INTERVAL");
        assert_eq!(poll_interval(), Duration::from_secs(300));
    }

    #[test]
    fn poll_interval_reads_from_env() {
        std::env::set_var("PKS_REMOTE_POLL_INTERVAL", "60");
        assert_eq!(poll_interval(), Duration::from_secs(60));
        std::env::remove_var("PKS_REMOTE_POLL_INTERVAL");
    }

    #[test]
    fn clone_remote_repo_returns_ok_if_dest_exists() {
        let dir = tempfile::TempDir::new().unwrap();
        let existing = dir.path().join("my-repo");
        std::fs::create_dir(&existing).unwrap();
        let result = clone_remote_repo("https://example.com/repo.git", dir.path(), "my-repo");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), existing);
    }

    #[test]
    fn fetch_non_existent_repo_returns_err() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = git_fetch(tmp.path());
        assert!(result.is_err(), "fetch on non-git dir should error");
    }
}
