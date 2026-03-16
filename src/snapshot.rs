use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;

pub const MAGIC: &[u8; 4] = b"PKS\0";
const VERSION: u32 = 1;
const SCHEMA_LAYOUT_ID: &str = "SnapshotData:v1:repo_id:chunks:vector_clock_sha:created_at_secs";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkRecord {
    pub file_path: String,
    pub heading_hierarchy: Vec<String>,
    pub chunk_index: usize,
    pub chunk_hash: String,
    pub chunk_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotData {
    pub repo_id: String,
    pub chunks: Vec<ChunkRecord>,
    pub vector_clock_sha: String,
    pub created_at_secs: u64,
}

#[derive(Debug)]
pub enum SnapshotError {
    Io(std::io::Error),
    VersionMismatch { found: u32, expected: u32 },
    SchemaMismatch,
    Corrupt(bincode::Error),
}

impl From<std::io::Error> for SnapshotError {
    fn from(e: std::io::Error) -> Self { SnapshotError::Io(e) }
}

pub struct SnapshotManager {
    snapshots_dir: PathBuf,
}

impl SnapshotManager {
    pub fn new_from_env() -> Self {
        let default = dirs_next_snapshot_dir();
        let dir = env::var("PKS_SNAPSHOTS_DIR").map(PathBuf::from).unwrap_or(default);
        SnapshotManager { snapshots_dir: dir }
    }

    /// Create a SnapshotManager with an explicit directory (useful for tests).
    pub fn new_with_dir(dir: PathBuf) -> Self {
        SnapshotManager { snapshots_dir: dir }
    }

    pub fn snapshot_file_path(&self, repo_id: &str) -> PathBuf {
        let safe_name = repo_id.replace(['/', '\\', ':'], "_");
        self.snapshots_dir.join(format!("{}.pks_snap", safe_name))
    }

    pub fn write_snapshot_for_repo(&self, data: &SnapshotData) -> std::io::Result<()> {
        fs::create_dir_all(&self.snapshots_dir)?;
        let bytes = serialize_snapshot_with_header(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(self.snapshot_file_path(&data.repo_id), bytes)
    }

    pub fn read_snapshot_for_repo(&self, repo_id: &str) -> Result<SnapshotData, SnapshotError> {
        let bytes = fs::read(self.snapshot_file_path(repo_id))?;
        deserialize_snapshot_validating_header(&bytes)
    }

    pub fn delete_snapshot_for_repo(&self, repo_id: &str) -> std::io::Result<()> {
        fs::remove_file(self.snapshot_file_path(repo_id))
    }
}

fn dirs_next_snapshot_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".pks").join("snapshots")
}

fn compute_schema_hash() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(SCHEMA_LAYOUT_ID.as_bytes());
    hasher.finalize().into()
}

fn serialize_snapshot_with_header(data: &SnapshotData) -> bincode::Result<Vec<u8>> {
    let payload = bincode::serialize(data)?;
    let schema_hash = compute_schema_hash();
    let mut out = Vec::with_capacity(4 + 4 + 32 + payload.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&schema_hash);
    out.extend_from_slice(&payload);
    Ok(out)
}

fn deserialize_snapshot_validating_header(bytes: &[u8]) -> Result<SnapshotData, SnapshotError> {
    let header_size = 4 + 4 + 32;
    if bytes.len() < header_size || &bytes[..4] != MAGIC {
        return Err(SnapshotError::SchemaMismatch);
    }
    let found_version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if found_version != VERSION {
        return Err(SnapshotError::VersionMismatch { found: found_version, expected: VERSION });
    }
    if bytes[8..40] != compute_schema_hash() {
        return Err(SnapshotError::SchemaMismatch);
    }
    bincode::deserialize(&bytes[header_size..]).map_err(SnapshotError::Corrupt)
}

pub fn compact_chunk_records(chunks: Vec<ChunkRecord>) -> Vec<ChunkRecord> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::with_capacity(chunks.len());
    for chunk in chunks.into_iter().rev() {
        if seen.insert(chunk.chunk_hash.clone()) {
            result.push(chunk);
        }
    }
    result.reverse();
    result
}

pub fn should_save_snapshot(commits_since_last_save: u64, secs_since_last_save: u64) -> bool {
    let interval_commits = env::var("PKS_SNAPSHOT_INTERVAL_COMMITS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(100);
    let interval_secs = env::var("PKS_SNAPSHOT_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(300);
    commits_since_last_save >= interval_commits || secs_since_last_save >= interval_secs
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_snapshot_data(repo_id: &str) -> SnapshotData {
        SnapshotData {
            repo_id: repo_id.to_string(),
            chunks: vec![ChunkRecord {
                file_path: "README.md".to_string(),
                heading_hierarchy: vec!["Intro".to_string()],
                chunk_index: 0,
                chunk_hash: "abc123".to_string(),
                chunk_text: "Hello world".to_string(),
            }],
            vector_clock_sha: "deadbeef".to_string(),
            created_at_secs: 1_700_000_000,
        }
    }

    fn manager_in(dir: &TempDir) -> SnapshotManager {
        SnapshotManager { snapshots_dir: dir.path().to_path_buf() }
    }

    #[test]
    fn write_then_read_produces_identical_snapshot_data() {
        let dir = TempDir::new().unwrap();
        let mgr = manager_in(&dir);
        let data = make_snapshot_data("repo-alpha");
        mgr.write_snapshot_for_repo(&data).unwrap();
        assert_eq!(mgr.read_snapshot_for_repo("repo-alpha").unwrap(), data);
    }

    #[test]
    fn version_mismatch_returns_error_without_panicking() {
        let dir = TempDir::new().unwrap();
        let mgr = manager_in(&dir);
        let data = make_snapshot_data("repo-beta");
        let mut bytes = serialize_snapshot_with_header(&data).unwrap();
        bytes[4..8].copy_from_slice(&9999u32.to_le_bytes());
        fs::write(mgr.snapshot_file_path("repo-beta"), &bytes).unwrap();
        assert!(matches!(
            mgr.read_snapshot_for_repo("repo-beta"),
            Err(SnapshotError::VersionMismatch { found: 9999, expected: 1 })
        ));
    }

    #[test]
    fn compact_chunk_records_removes_duplicate_hashes() {
        let make = |hash: &str, idx: usize| ChunkRecord {
            file_path: "a.md".to_string(),
            heading_hierarchy: vec![],
            chunk_index: idx,
            chunk_hash: hash.to_string(),
            chunk_text: "t".to_string(),
        };
        let compacted = compact_chunk_records(vec![make("h1", 0), make("h2", 1), make("h1", 2)]);
        assert_eq!(compacted.len(), 2);
        let hashes: Vec<_> = compacted.iter().map(|c| c.chunk_hash.as_str()).collect();
        assert!(hashes.contains(&"h1") && hashes.contains(&"h2"));
    }
}
