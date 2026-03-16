pub mod lfs_store;

use crate::snapshot::SnapshotData;
use std::env;
use std::fs;
use std::path::PathBuf;

pub use lfs_store::GitLfsStore;

#[derive(Debug)]
pub enum SyncError {
    Io(std::io::Error),
    Serialization(bincode::Error),
    GitCommand(String),
    NotConfigured,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Io(e) => write!(f, "IO error: {}", e),
            SyncError::Serialization(e) => write!(f, "Serialization error: {}", e),
            SyncError::GitCommand(msg) => write!(f, "Git command failed: {}", msg),
            SyncError::NotConfigured => write!(f, "PKS_VECTOR_REMOTE_URL not configured"),
        }
    }
}

impl From<std::io::Error> for SyncError {
    fn from(e: std::io::Error) -> Self {
        SyncError::Io(e)
    }
}

impl From<bincode::Error> for SyncError {
    fn from(e: bincode::Error) -> Self {
        SyncError::Serialization(e)
    }
}

pub trait SnapshotStore: Send + Sync {
    fn save_snapshot(&self, data: &SnapshotData) -> Result<(), SyncError>;
    fn load_snapshot(&self, repo_id: &str) -> Result<SnapshotData, SyncError>;
    fn sync_snapshot(&self, data: &SnapshotData) -> Result<(), SyncError>;
}

pub struct LocalStore {
    pub(super) snapshots_dir: PathBuf,
}

impl LocalStore {
    pub fn new_from_env() -> Self {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let default_dir = PathBuf::from(home).join(".pks").join("snapshots");
        let dir = env::var("PKS_SNAPSHOTS_DIR")
            .map(PathBuf::from)
            .unwrap_or(default_dir);
        LocalStore { snapshots_dir: dir }
    }

    pub fn new_with_dir(dir: PathBuf) -> Self {
        LocalStore { snapshots_dir: dir }
    }

    fn snapshot_path(&self, repo_id: &str) -> PathBuf {
        let safe = repo_id.replace(['/', '\\', ':'], "_");
        self.snapshots_dir.join(format!("{}.bin", safe))
    }
}

impl SnapshotStore for LocalStore {
    fn save_snapshot(&self, data: &SnapshotData) -> Result<(), SyncError> {
        fs::create_dir_all(&self.snapshots_dir)?;
        let bytes = bincode::serialize(data)?;
        fs::write(self.snapshot_path(&data.repo_id), bytes)?;
        Ok(())
    }

    fn load_snapshot(&self, repo_id: &str) -> Result<SnapshotData, SyncError> {
        let bytes = fs::read(self.snapshot_path(repo_id))?;
        let data = bincode::deserialize(&bytes)?;
        Ok(data)
    }

    fn sync_snapshot(&self, data: &SnapshotData) -> Result<(), SyncError> {
        self.save_snapshot(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{ChunkRecord, SnapshotData};
    use tempfile::TempDir;

    fn make_snapshot(repo_id: &str) -> SnapshotData {
        SnapshotData {
            repo_id: repo_id.to_string(),
            chunks: vec![ChunkRecord {
                file_path: "README.md".to_string(),
                heading_hierarchy: vec![],
                chunk_index: 0,
                chunk_hash: "abc".to_string(),
                chunk_text: "hello".to_string(),
            }],
            vector_clock_sha: "deadbeef".to_string(),
            created_at_secs: 1_700_000_000,
        }
    }

    #[test]
    fn local_store_save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let store = LocalStore::new_with_dir(dir.path().to_path_buf());
        let data = make_snapshot("test-repo");
        store.save_snapshot(&data).unwrap();
        let loaded = store.load_snapshot("test-repo").unwrap();
        assert_eq!(loaded.repo_id, data.repo_id);
        assert_eq!(loaded.vector_clock_sha, data.vector_clock_sha);
    }

    #[test]
    fn local_store_load_missing_repo_returns_error() {
        let dir = TempDir::new().unwrap();
        let store = LocalStore::new_with_dir(dir.path().to_path_buf());
        assert!(store.load_snapshot("nonexistent").is_err());
    }

    #[test]
    fn local_store_sync_is_equivalent_to_save() {
        let dir = TempDir::new().unwrap();
        let store = LocalStore::new_with_dir(dir.path().to_path_buf());
        let data = make_snapshot("sync-repo");
        store.sync_snapshot(&data).unwrap();
        let loaded = store.load_snapshot("sync-repo").unwrap();
        assert_eq!(loaded.repo_id, data.repo_id);
    }
}
