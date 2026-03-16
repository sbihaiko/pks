use std::path::Path;

use crate::indexer::pipeline::IndexingPipeline;
use crate::memory_manager::MemoryManager;
use crate::search::retriever::{SearchBackend, TantivyBackend};
use crate::snapshot::{SnapshotError, SnapshotManager};
use crate::state::VectorClock;
use crate::recovery_reconcile::{restore_chunks_from_snapshot, reconcile_with_head};

#[derive(Debug)]
pub enum RecoveryOutcome {
    RestoredFromSnapshot,
    RebuiltFromHead,
    DroppedAndRebuilt,
}

pub struct RecoveryEngine<'a> {
    snapshot_manager: &'a SnapshotManager,
    pipeline: &'a IndexingPipeline,
}

impl<'a> RecoveryEngine<'a> {
    pub fn new(snapshot_manager: &'a SnapshotManager, pipeline: &'a IndexingPipeline) -> Self {
        Self { snapshot_manager, pipeline }
    }

    pub fn recover_repo(
        &self,
        repo_id: &str,
        repo_path: &Path,
        backend: &mut TantivyBackend,
        vector_clock: &mut VectorClock,
    ) -> RecoveryOutcome {
        match self.snapshot_manager.read_snapshot_for_repo(repo_id) {
            Ok(snapshot) => {
                restore_chunks_from_snapshot(repo_id, &snapshot, backend);
                reconcile_with_head(repo_id, repo_path, &snapshot, self.pipeline, backend);
                vector_clock.update(repo_id, "main", &snapshot.vector_clock_sha);
                RecoveryOutcome::RestoredFromSnapshot
            }
            Err(SnapshotError::VersionMismatch { .. }) | Err(SnapshotError::SchemaMismatch) => {
                let _ = self.snapshot_manager.delete_snapshot_for_repo(repo_id);
                self.rebuild_from_head(repo_id, repo_path, backend);
                RecoveryOutcome::RebuiltFromHead
            }
            Err(SnapshotError::Io(_)) => {
                self.rebuild_from_head(repo_id, repo_path, backend);
                RecoveryOutcome::RebuiltFromHead
            }
            Err(SnapshotError::Corrupt(_)) => {
                let _ = self.snapshot_manager.delete_snapshot_for_repo(repo_id);
                self.rebuild_from_head(repo_id, repo_path, backend);
                RecoveryOutcome::RebuiltFromHead
            }
        }
    }

    pub fn handle_rebase(
        &self,
        repo_id: &str,
        repo_path: &Path,
        branch: &str,
        backend: &mut TantivyBackend,
        vector_clock: &mut VectorClock,
        snapshot_manager: &SnapshotManager,
    ) -> RecoveryOutcome {
        let _ = backend.remove_chunks_for_repo(repo_id);
        let _ = backend.commit();
        let _ = snapshot_manager.delete_snapshot_for_repo(repo_id);
        vector_clock.remove_repo(repo_id);
        self.rebuild_from_head(repo_id, repo_path, backend);
        vector_clock.update(repo_id, branch, "rebuilt");
        RecoveryOutcome::DroppedAndRebuilt
    }

    pub fn rebuild_from_head(
        &self,
        repo_id: &str,
        repo_path: &Path,
        backend: &mut TantivyBackend,
    ) {
        MemoryManager::load_repo(repo_id, repo_path, self.pipeline, backend);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_repo(dir: &TempDir) -> std::path::PathBuf {
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(
            repo.join("note.md"),
            "# Test\n\nThis is a test document for recovery.",
        )
        .unwrap();
        repo
    }

    #[test]
    fn rebuild_from_head_indexes_md_files() {
        let dir = TempDir::new().unwrap();
        let repo = make_test_repo(&dir);
        let snap_dir = dir.path().join("snaps");
        std::fs::create_dir_all(&snap_dir).unwrap();

        let snap_mgr = SnapshotManager::new_with_dir(snap_dir);
        let pipeline = IndexingPipeline::new_from_env();
        let engine = RecoveryEngine::new(&snap_mgr, &pipeline);

        let mut backend = TantivyBackend::new_in_memory().unwrap();
        engine.rebuild_from_head("repo-test", &repo, &mut backend);

        let results = backend.search("test document recovery", 5, None).unwrap();
        assert!(!results.is_empty(), "should find content after rebuild");
    }

    #[test]
    fn recover_repo_absent_snapshot_triggers_rebuild() {
        let dir = TempDir::new().unwrap();
        let repo = make_test_repo(&dir);
        let snap_dir = dir.path().join("snaps");
        std::fs::create_dir_all(&snap_dir).unwrap();

        let snap_mgr = SnapshotManager::new_with_dir(snap_dir);
        let pipeline = IndexingPipeline::new_from_env();
        let engine = RecoveryEngine::new(&snap_mgr, &pipeline);

        let mut backend = TantivyBackend::new_in_memory().unwrap();
        let mut vc = VectorClock::default();

        let outcome = engine.recover_repo("repo-r", &repo, &mut backend, &mut vc);
        assert!(matches!(outcome, RecoveryOutcome::RebuiltFromHead));

        let results = backend.search("test document", 5, None).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn handle_rebase_clears_clock_and_rebuilds() {
        let dir = TempDir::new().unwrap();
        let repo = make_test_repo(&dir);
        let snap_dir = dir.path().join("snaps");
        std::fs::create_dir_all(&snap_dir).unwrap();

        let snap_mgr = SnapshotManager::new_with_dir(snap_dir.clone());
        let pipeline = IndexingPipeline::new_from_env();
        let engine = RecoveryEngine::new(&snap_mgr, &pipeline);

        let mut backend = TantivyBackend::new_in_memory().unwrap();
        let mut vc = VectorClock::default();
        vc.update("repo-rb", "main", "old-sha");

        let snap_mgr2 = SnapshotManager::new_with_dir(snap_dir);
        let outcome = engine.handle_rebase(
            "repo-rb", &repo, "main",
            &mut backend, &mut vc, &snap_mgr2,
        );

        assert!(matches!(outcome, RecoveryOutcome::DroppedAndRebuilt));
        assert_eq!(vc.get("repo-rb", "main"), Some(&"rebuilt".to_string()));
    }

    #[test]
    fn delete_snapshots_then_recover_produces_equivalent_index() {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join("a.md"), "# Alpha\n\nContent about alpha systems.").unwrap();
        std::fs::write(repo.join("b.md"), "# Beta\n\nContent about beta testing.").unwrap();

        let snap_dir = dir.path().join("snaps");
        std::fs::create_dir_all(&snap_dir).unwrap();

        let pipeline = IndexingPipeline::new_from_env();
        let snap_mgr = SnapshotManager::new_with_dir(snap_dir.clone());
        let engine = RecoveryEngine::new(&snap_mgr, &pipeline);

        let mut original = TantivyBackend::new_in_memory().unwrap();
        MemoryManager::load_repo("repo-eq", &repo, &pipeline, &mut original);

        let mut rebuilt = TantivyBackend::new_in_memory().unwrap();
        engine.rebuild_from_head("repo-eq", &repo, &mut rebuilt);

        let r1 = original.search("alpha", 5, None).unwrap();
        let r2 = rebuilt.search("alpha", 5, None).unwrap();
        assert!(!r1.is_empty() && !r2.is_empty(), "both should find alpha");
        assert_eq!(r1[0].file_path, r2[0].file_path, "same file from both indices");
    }
}
