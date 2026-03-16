use std::collections::HashMap;
use std::time::{Instant, SystemTime};
use serde::{Deserialize, Serialize};

pub type RepoId = String;
pub type Branch = String;
pub type CommitSha = String;

pub enum PipelineEvent {
    FileChanged { repo_id: RepoId, file_path: String, content: String },
    FileDeleted { repo_id: RepoId, file_path: String },
    RepoRegistered { repo_id: RepoId, path: std::path::PathBuf },
    RepoDeregistered { repo_id: RepoId },
}

pub struct RawTransaction {
    pub event: PipelineEvent,
    pub commit_sha: Option<CommitSha>,
    pub tree_hash: Option<String>,
    pub branch: Option<Branch>,
    pub ingested_at: std::time::Instant,
}

pub enum IndexMutation {
    AddChunks(Vec<(crate::indexer::chunker::Chunk, Option<Vec<f32>>)>),
    RemoveFile { repo_id: RepoId, file_path: String },
}

pub struct StampedMutation {
    pub mutation: IndexMutation,
    pub ingested_at: SystemTime,
    pub repo_id: RepoId,
    pub branch: Branch,
    pub commit_sha: Option<CommitSha>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub repo_id: RepoId,
    pub file_path: String,
    pub heading_hierarchy: Vec<String>,
    pub chunk_index: usize,
    pub chunk_hash: String,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingDebtEntry {
    pub repo_id: RepoId,
    pub file_path: String,
    pub chunk_index: usize,
    pub chunk_hash: String,
    pub chunk_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorClock {
    pub clocks: HashMap<(RepoId, Branch), CommitSha>,
}

impl VectorClock {
    pub fn update(&mut self, repo_id: &str, branch: &str, commit_sha: &str) {
        self.clocks.insert(
            (repo_id.to_string(), branch.to_string()),
            commit_sha.to_string(),
        );
    }

    pub fn get(&self, repo_id: &str, branch: &str) -> Option<&CommitSha> {
        self.clocks.get(&(repo_id.to_string(), branch.to_string()))
    }

    pub fn is_potential_rebase(&self, repo_id: &str, branch: &str, new_sha: &str) -> bool {
        self.get(repo_id, branch).is_some_and(|recorded| recorded != new_sha)
    }

    pub fn remove_repo(&mut self, repo_id: &str) {
        self.clocks.retain(|(rid, _), _| rid != repo_id);
    }

    pub fn tracked_branches(&self) -> Vec<(&RepoId, &Branch)> {
        self.clocks.keys().map(|(r, b)| (r, b)).collect()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GlobalStats {
    pub total_chunks_indexed: u64,
    pub total_queries_served: u64,
    pub total_commits_processed: u64,
    pub pks_embedding_debt_entries: u64,
}

#[derive(Debug)]
pub struct RepoIndex {
    pub repo_id: RepoId,
    pub chunk_count: usize,
}

pub struct PrevalentState {
    pub repos: HashMap<RepoId, RepoIndex>,
    pub vector_clock: VectorClock,
    pub embedding_debt: Vec<EmbeddingDebtEntry>,
    pub global_stats: GlobalStats,
    pub search_index: crate::search::retriever::TantivyBackend,
    pub started_at: Instant,
}

impl Default for PrevalentState {
    fn default() -> Self {
        Self {
            repos: HashMap::new(),
            vector_clock: VectorClock::default(),
            embedding_debt: Vec::new(),
            global_stats: GlobalStats::default(),
            search_index: crate::search::retriever::TantivyBackend::new_in_memory()
                .expect("tantivy in-memory index must initialize"),
            started_at: Instant::now(),
        }
    }
}

impl PrevalentState {
    pub fn list_repo_ids(&self) -> Vec<String> {
        self.repos.keys().cloned().collect()
    }

    pub fn save_all_snapshots(&self) -> std::io::Result<()> {
        use crate::snapshot::{SnapshotData, SnapshotManager, ChunkRecord};
        let mgr = SnapshotManager::new_from_env();
        for repo_id in self.repos.keys() {
            let chunks: Vec<ChunkRecord> = self.search_index.vectors.iter()
                .filter(|_| true) // simplificado por enquanto; idealmente filtra pelo repo_id se o Tantivy suportasse
                .map(|(text, _vec)| ChunkRecord {
                    file_path: "unknown".to_string(), // No MVP, o mapeamento 1:1 de chunk -> file pode estar simplificado
                    heading_hierarchy: vec![],
                    chunk_index: 0,
                    chunk_hash: "".to_string(),
                    chunk_text: text.clone(),
                })
                .collect();

            let data = SnapshotData {
                repo_id: repo_id.clone(),
                chunks,
                vector_clock_sha: "".to_string(), // Ajustar conforme vector_clock evoluir
                created_at_secs: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            };
            mgr.write_snapshot_for_repo(&data)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_clock_update_and_get() {
        let mut vc = VectorClock::default();
        vc.update("repo-a", "main", "sha1");
        vc.update("repo-a", "feature/x", "sha2");
        vc.update("repo-b", "main", "sha3");

        assert_eq!(vc.get("repo-a", "main"), Some(&"sha1".to_string()));
        assert_eq!(vc.get("repo-a", "feature/x"), Some(&"sha2".to_string()));
        assert_eq!(vc.get("repo-b", "main"), Some(&"sha3".to_string()));
        assert_eq!(vc.get("repo-c", "main"), None);
    }

    #[test]
    fn vector_clock_three_repos_multiple_branches_deterministic() {
        let repos = [
            ("repo-a", "main", "aaaa"),
            ("repo-a", "feature/1", "aaab"),
            ("repo-b", "main", "bbbb"),
            ("repo-b", "feature/2", "bbbc"),
            ("repo-c", "main", "cccc"),
            ("repo-c", "pks-knowledge", "cccd"),
        ];

        let mut vc1 = VectorClock::default();
        let mut vc2 = VectorClock::default();

        for (r, b, sha) in &repos {
            vc1.update(r, b, sha);
        }
        for (r, b, sha) in repos.iter().rev() {
            vc2.update(r, b, sha);
        }

        assert_eq!(vc1.clocks, vc2.clocks, "order of updates must not affect final state");
    }

    #[test]
    fn vector_clock_is_potential_rebase_on_sha_change() {
        let mut vc = VectorClock::default();
        vc.update("repo-a", "main", "sha_old");
        assert!(vc.is_potential_rebase("repo-a", "main", "sha_new"));
        assert!(!vc.is_potential_rebase("repo-a", "main", "sha_old"));
        assert!(!vc.is_potential_rebase("repo-z", "main", "anything"));
    }

    #[test]
    fn vector_clock_remove_repo_clears_all_branches() {
        let mut vc = VectorClock::default();
        vc.update("repo-a", "main", "s1");
        vc.update("repo-a", "dev", "s2");
        vc.update("repo-b", "main", "s3");
        vc.remove_repo("repo-a");
        assert_eq!(vc.get("repo-a", "main"), None);
        assert_eq!(vc.get("repo-a", "dev"), None);
        assert!(vc.get("repo-b", "main").is_some());
    }
}
