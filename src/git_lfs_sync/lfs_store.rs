use super::{LocalStore, SnapshotStore, SyncError};
use crate::snapshot::SnapshotData;
use std::env;
use std::path::PathBuf;
use std::process::Command;

pub struct GitLfsStore {
    local: LocalStore,
    remote_url: String,
    lfs_repo_dir: PathBuf,
    compress: bool,
}

impl GitLfsStore {
    pub fn new_from_env() -> Result<Self, SyncError> {
        let remote_url = env::var("PKS_VECTOR_REMOTE_URL")
            .map_err(|_| SyncError::NotConfigured)?;
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let lfs_repo_dir = PathBuf::from(&home).join(".pks").join("lfs-snapshots");
        let compress = env::var("PKS_BACKUP_COMPRESS")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let local = LocalStore::new_from_env();
        Ok(GitLfsStore { local, remote_url, lfs_repo_dir, compress })
    }

    fn ensure_lfs_repo_initialized(&self) -> Result<(), SyncError> {
        if self.lfs_repo_dir.join(".git").exists() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.lfs_repo_dir)?;
        run_git_command(&self.lfs_repo_dir, &["init"])?;
        run_git_command(&self.lfs_repo_dir, &["lfs", "track", "*.bin"])?;
        write_gitattributes(&self.lfs_repo_dir)?;
        run_git_command(&self.lfs_repo_dir, &["remote", "add", "origin", &self.remote_url])?;
        Ok(())
    }

    fn stage_and_push_snapshot(&self, repo_id: &str) -> Result<(), SyncError> {
        let safe = repo_id.replace(['/', '\\', ':'], "_");
        let filename = format!("{}.bin", safe);
        let src = self.local.snapshots_dir.join(&filename);
        let dst = self.lfs_repo_dir.join(&filename);
        std::fs::copy(&src, &dst)?;
        run_git_command(&self.lfs_repo_dir, &["add", &filename])?;
        let msg = format!("snapshot: {}", safe);
        run_git_command(&self.lfs_repo_dir, &["commit", "--allow-empty", "-m", &msg])?;
        run_git_command(&self.lfs_repo_dir, &["push", "--force", "origin", "HEAD:snapshots"])?;
        Ok(())
    }
}

impl SnapshotStore for GitLfsStore {
    fn save_snapshot(&self, data: &SnapshotData) -> Result<(), SyncError> {
        self.local.save_snapshot(data)
    }

    fn load_snapshot(&self, repo_id: &str) -> Result<SnapshotData, SyncError> {
        self.local.load_snapshot(repo_id)
    }

    fn sync_snapshot(&self, data: &SnapshotData) -> Result<(), SyncError> {
        self.local.save_snapshot(data)?;
        if self.compress {
            tracing::warn!("zstd compression not yet implemented");
        }
        let init_result = self.ensure_lfs_repo_initialized();
        if let Err(e) = init_result {
            tracing::warn!("LFS repo init failed (non-fatal): {}", e);
            return Ok(());
        }
        let push_result = self.stage_and_push_snapshot(&data.repo_id);
        if let Err(e) = push_result {
            tracing::warn!("LFS push failed (non-fatal): {}", e);
        }
        Ok(())
    }
}

fn run_git_command(dir: &PathBuf, args: &[&str]) -> Result<(), SyncError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| SyncError::GitCommand(e.to_string()))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Err(SyncError::GitCommand(stderr))
}

fn write_gitattributes(dir: &PathBuf) -> Result<(), SyncError> {
    let path = dir.join(".gitattributes");
    let content = "*.bin filter=lfs diff=lfs merge=lfs -text\n";
    std::fs::write(path, content)?;
    Ok(())
}
