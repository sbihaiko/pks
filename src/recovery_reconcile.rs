use std::collections::HashSet;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::indexer::chunker::Chunk;
use crate::indexer::pipeline::IndexingPipeline;
use crate::search::retriever::{SearchBackend, TantivyBackend};
use crate::snapshot::SnapshotData;

pub fn restore_chunks_from_snapshot(
    repo_id: &str,
    snapshot: &SnapshotData,
    backend: &mut TantivyBackend,
) {
    for record in &snapshot.chunks {
        let chunk = Chunk {
            repo_id: repo_id.to_string(),
            file_path: record.file_path.clone(),
            heading_hierarchy: record.heading_hierarchy.clone(),
            chunk_index: record.chunk_index,
            chunk_hash: record.chunk_hash.clone(),
            text: record.chunk_text.clone(),
            is_tombstone: false,
        };
        let _ = backend.add_chunk(&chunk);
    }
    let _ = backend.commit();
}

fn needs_reindex(rel: &str, content_hash: &str, snapshot: &SnapshotData) -> bool {
    let in_snapshot = snapshot.chunks.iter().any(|c| c.file_path == rel);
    let hash_matches = snapshot
        .chunks
        .iter()
        .any(|c| c.file_path == rel && c.chunk_hash == content_hash);
    !in_snapshot || !hash_matches
}

fn reindex_file(
    repo_id: &str,
    rel: &str,
    content: &str,
    pipeline: &IndexingPipeline,
    backend: &mut TantivyBackend,
) {
    let _ = backend.remove_chunks_for_file(repo_id, rel);
    for chunk in pipeline.process_file(repo_id, rel, content) {
        let _ = backend.add_chunk(&chunk);
    }
}

fn content_hash(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn rel_path(file_path: &Path, repo_path: &Path) -> String {
    file_path
        .strip_prefix(repo_path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| file_path.to_string_lossy().into_owned())
}

fn reindex_changed_files(
    repo_id: &str,
    repo_path: &Path,
    snapshot: &SnapshotData,
    pipeline: &IndexingPipeline,
    backend: &mut TantivyBackend,
) -> bool {
    let md_files = crate::memory_manager::collect_md_files_pub(repo_path);
    let mut did_write = false;
    for file_path in &md_files {
        let Ok(content) = std::fs::read_to_string(file_path) else {
            continue;
        };
        let rel = rel_path(file_path, repo_path);
        let hash = content_hash(&content);
        if !needs_reindex(&rel, &hash, snapshot) {
            continue;
        }
        reindex_file(repo_id, &rel, &content, pipeline, backend);
        did_write = true;
    }
    did_write
}

fn remove_deleted_snapshot_files(
    repo_id: &str,
    repo_path: &Path,
    snapshot_paths: HashSet<&str>,
    backend: &mut TantivyBackend,
) -> bool {
    let mut did_write = false;
    for path in snapshot_paths {
        let full = repo_path.join(path);
        if full.exists() {
            continue;
        }
        let _ = backend.remove_chunks_for_file(repo_id, path);
        did_write = true;
    }
    did_write
}

pub fn reconcile_with_head(
    repo_id: &str,
    repo_path: &Path,
    snapshot: &SnapshotData,
    pipeline: &IndexingPipeline,
    backend: &mut TantivyBackend,
) {
    let snapshot_paths: HashSet<&str> =
        snapshot.chunks.iter().map(|c| c.file_path.as_str()).collect();

    let wrote_reindex = reindex_changed_files(repo_id, repo_path, snapshot, pipeline, backend);
    let wrote_delete =
        remove_deleted_snapshot_files(repo_id, repo_path, snapshot_paths, backend);

    if wrote_reindex || wrote_delete {
        let _ = backend.commit();
    }
}
