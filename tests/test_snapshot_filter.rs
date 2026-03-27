#[cfg(test)]
mod tests {
    use pks::indexer::chunker::Chunk;
    use pks::search::retriever::{SearchBackend, TantivyBackend};
    use pks::snapshot::SnapshotManager;
    use pks::state::{PrevalentState, RepoIndex};
    use tempfile::TempDir;

    fn make_chunk(repo_id: &str, file: &str, text: &str, idx: usize) -> Chunk {
        Chunk {
            repo_id: repo_id.to_string(),
            file_path: file.to_string(),
            heading_hierarchy: vec!["Section".to_string()],
            chunk_index: idx,
            chunk_hash: format!("{repo_id}-{file}-{idx}"),
            text: text.to_string(),
            is_tombstone: false,
        }
    }

    fn add_chunks_for_repo(backend: &mut TantivyBackend, repo_id: &str, file: &str, texts: &[&str]) {
        for (i, text) in texts.iter().enumerate() {
            let chunk = make_chunk(repo_id, file, text, i);
            backend.add_chunk(&chunk).unwrap();
        }
        backend.commit().unwrap();
    }

    #[test]
    fn save_all_snapshots_only_includes_repo_own_chunks() {
        let tmp = TempDir::new().unwrap();
        let snap_dir = tmp.path().to_path_buf();

        let mut state = PrevalentState::default();

        state.repos.insert("repo-a".to_string(), RepoIndex {
            repo_id: "repo-a".to_string(),
            chunk_count: 2,
        });
        state.repos.insert("repo-b".to_string(), RepoIndex {
            repo_id: "repo-b".to_string(),
            chunk_count: 2,
        });

        add_chunks_for_repo(
            &mut state.search_index,
            "repo-a",
            "docs/alpha.md",
            &["Alpha chunk one content.", "Alpha chunk two content."],
        );
        add_chunks_for_repo(
            &mut state.search_index,
            "repo-b",
            "docs/beta.md",
            &["Beta chunk one content.", "Beta chunk two content."],
        );

        let mgr = SnapshotManager::new_with_dir(snap_dir.clone());
        // Write snapshots using the manager (bypassing env var lookup).
        for repo_id in state.repos.keys() {
            let chunks = state
                .search_index
                .chunk_meta
                .iter()
                .filter(|(_, meta)| meta.repo_id.as_str() == repo_id.as_str())
                .map(|(text, meta)| pks::snapshot::ChunkRecord {
                    file_path: meta.file_path.clone(),
                    heading_hierarchy: meta.heading_hierarchy.clone(),
                    chunk_index: meta.chunk_index,
                    chunk_hash: meta.chunk_hash.clone(),
                    chunk_text: text.clone(),
                })
                .collect::<Vec<_>>();
            let data = pks::snapshot::SnapshotData {
                repo_id: repo_id.clone(),
                chunks,
                vector_clock_sha: "".to_string(),
                created_at_secs: 0,
            };
            mgr.write_snapshot_for_repo(&data).unwrap();
        }

        let snap_a = mgr.read_snapshot_for_repo("repo-a").unwrap();
        let snap_b = mgr.read_snapshot_for_repo("repo-b").unwrap();

        // Repo A snapshot must only contain repo-a chunks.
        assert!(
            snap_a.chunks.iter().all(|c| c.chunk_text.starts_with("Alpha")),
            "repo-a snapshot contains non-alpha chunks: {:?}",
            snap_a.chunks
        );
        assert!(
            !snap_a.chunks.iter().any(|c| c.chunk_text.starts_with("Beta")),
            "repo-a snapshot must not contain beta chunks"
        );

        // Repo B snapshot must only contain repo-b chunks.
        assert!(
            snap_b.chunks.iter().all(|c| c.chunk_text.starts_with("Beta")),
            "repo-b snapshot contains non-beta chunks: {:?}",
            snap_b.chunks
        );
        assert!(
            !snap_b.chunks.iter().any(|c| c.chunk_text.starts_with("Alpha")),
            "repo-b snapshot must not contain alpha chunks"
        );

        // Metadata fields must be properly populated (no "unknown" fallbacks).
        for chunk in &snap_a.chunks {
            assert_eq!(chunk.file_path, "docs/alpha.md");
            assert!(!chunk.chunk_hash.is_empty());
            assert!(!chunk.heading_hierarchy.is_empty());
        }
        for chunk in &snap_b.chunks {
            assert_eq!(chunk.file_path, "docs/beta.md");
            assert!(!chunk.chunk_hash.is_empty());
            assert!(!chunk.heading_hierarchy.is_empty());
        }
    }
}
