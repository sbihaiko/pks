use std::path::Path;

use crate::indexer::pipeline::IndexingPipeline;
use crate::search::retriever::{SearchBackend, TantivyBackend};
use crate::snapshot::SnapshotManager;

pub struct MemoryManager;

impl MemoryManager {
    pub fn load_repo(
        repo_id: &str,
        repo_path: &Path,
        pipeline: &IndexingPipeline,
        backend: &mut TantivyBackend,
    ) -> usize {
        let md_files = collect_md_files(repo_path);
        let mut chunk_count = 0;

        for file_path in &md_files {
            let Ok(content) = std::fs::read_to_string(file_path) else {
                continue;
            };
            let rel = file_path
                .strip_prefix(repo_path)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| file_path.to_string_lossy().into_owned());

            let chunks = pipeline.process_file(repo_id, &rel, &content);
            chunk_count += chunks.len();
            for chunk in &chunks {
                let _ = backend.add_chunk(chunk);
            }
        }

        let _ = backend.commit();
        chunk_count
    }

    pub fn unload_repo(
        repo_id: &str,
        backend: &mut TantivyBackend,
        snapshot_manager: &SnapshotManager,
    ) {
        let _ = remove_all_chunks_for_repo(repo_id, backend);
        let _ = backend.commit();
        let _ = snapshot_manager.delete_snapshot_for_repo(repo_id);
    }
}

pub fn collect_md_files_pub(dir: &Path) -> Vec<std::path::PathBuf> {
    collect_md_files(dir)
}

fn collect_md_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut result = Vec::new();
    collect_md_files_inner(dir, &mut result);
    result
}

fn is_hidden_dir(path: &std::path::Path) -> bool {
    path.is_dir()
        && path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
}

fn collect_md_files_inner(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if is_hidden_dir(&path) {
            continue;
        }
        if path.is_dir() {
            collect_md_files_inner(&path, out);
            continue;
        }
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            out.push(path);
        }
    }
}

fn remove_all_chunks_for_repo(
    repo_id: &str,
    backend: &mut TantivyBackend,
) -> tantivy::Result<()> {
    backend.remove_chunks_for_repo(repo_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_repo(dir: &TempDir, files: &[(&str, &str)]) -> std::path::PathBuf {
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        for (name, content) in files {
            std::fs::write(repo.join(name), content).unwrap();
        }
        repo
    }

    #[test]
    fn load_repo_indexes_md_files() {
        let dir = TempDir::new().unwrap();
        let repo = make_repo(&dir, &[
            ("README.md", "# Hello\n\nThis is a test note about indexing."),
            ("notes.md", "# Notes\n\nSome content here."),
            ("ignored.txt", "not markdown"),
        ]);

        let pipeline = IndexingPipeline::new_from_env();
        let mut backend = TantivyBackend::new_in_memory().unwrap();

        let count = MemoryManager::load_repo("test-repo", &repo, &pipeline, &mut backend);
        assert!(count >= 2, "expected at least 2 chunks from 2 md files, got {count}");
    }

    #[test]
    fn unload_repo_deletes_snapshot() {
        let dir = TempDir::new().unwrap();
        let snap_mgr = SnapshotManager::new_with_dir(dir.path().to_path_buf());

        let data = crate::snapshot::SnapshotData {
            repo_id: "repo-x".to_string(),
            chunks: vec![],
            vector_clock_sha: "abc".to_string(),
            created_at_secs: 0,
        };
        snap_mgr.write_snapshot_for_repo(&data).unwrap();
        assert!(snap_mgr.snapshot_file_path("repo-x").exists());

        let mut backend = TantivyBackend::new_in_memory().unwrap();
        MemoryManager::unload_repo("repo-x", &mut backend, &snap_mgr);

        assert!(!snap_mgr.snapshot_file_path("repo-x").exists());
    }

    #[test]
    fn collect_md_skips_hidden_dirs() {
        let dir = TempDir::new().unwrap();
        let base = dir.path();
        std::fs::create_dir_all(base.join(".git")).unwrap();
        std::fs::write(base.join(".git/COMMIT_EDITMSG"), "ignore me").unwrap();
        std::fs::write(base.join("visible.md"), "# Visible").unwrap();

        let files = collect_md_files(base);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("visible.md"));
    }
}
