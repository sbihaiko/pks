use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::git::RepoIdentity;
use crate::indexer::pipeline::IndexingPipeline;
use crate::repo_watcher::RepoWatcher;
use crate::search::retriever::SearchBackend;
use crate::state::{PrevalentState, RepoIndex};

pub const VAULT_DIR_NAME: &str = "prometheus";

fn collect_md_entry(path: PathBuf, out: &mut Vec<PathBuf>) {
    // Ignore junk directories early for performance and to avoid indexing vendor files
    if path.is_dir() {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.')
                || name == "node_modules"
                || name == "target"
                || name == "vendor"
                || name == "venv"
                || name == ".venv"
                || name == VAULT_DIR_NAME
            {
                return;
            }
        }
        walk_md_files(&path, out);
        return;
    }
    if path.extension().is_some_and(|ext| ext == "md") {
        out.push(path);
    }
}

pub fn walk_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.filter_map(|e| e.ok()) {
        collect_md_entry(entry.path(), out);
    }
}

pub async fn ingest_file_chunks(
    repo_id: &str,
    file_path: &Path,
    pipeline: &mut IndexingPipeline,
    state: &Arc<Mutex<PrevalentState>>,
) {
    use crate::embedding_provider::{EmbeddingProvider, EmbeddingProviderKind, OllamaProvider};
    let Ok(content) = std::fs::read_to_string(file_path) else { return };
    let file_str = file_path.to_string_lossy().into_owned();
    let tagged_chunks = pipeline.process_file_with_dirty_markers(repo_id, &file_str, &content);
    let provider_kind = EmbeddingProviderKind::from_env();
    let embedder = provider_kind.is_ollama().then(OllamaProvider::from_env);
    let mut results = Vec::new();
    for (chunk, is_dirty) in tagged_chunks {
        let vec = if is_dirty {
            if let Some(ref e) = embedder { e.embed_text(&chunk.text).await.ok() } else { None }
        } else {
            None
        };
        results.push((chunk, vec));
    }
    let mut guard = state.lock().unwrap();
    for (chunk, vector) in results {
        if let Some(v) = vector {
            let _ = guard.search_index.add_chunk_with_vector(&chunk, v);
        } else {
            let _ = guard.search_index.add_chunk(&chunk);
        }
    }
}

pub async fn index_repo(
    repo_path: &Path,
    pipeline: &mut IndexingPipeline,
    state: &Arc<Mutex<PrevalentState>>,
) {
    let repo_id = RepoIdentity::from_path(repo_path)
        .map(|id| id.repo_id)
        .unwrap_or_else(|_| {
            repo_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
    let mut markdown_file_paths = Vec::new();
    walk_md_files(repo_path, &mut markdown_file_paths);
    for file_path in &markdown_file_paths {
        ingest_file_chunks(&repo_id, file_path, pipeline, state).await;
    }
    let mut guard = state.lock().unwrap();
    guard.repos.insert(
        repo_id.clone(),
        RepoIndex { repo_id, chunk_count: markdown_file_paths.len() },
    );
}

pub async fn index_vault_worktree(
    repo_path: &Path,
    pipeline: &mut IndexingPipeline,
    state: &Arc<Mutex<PrevalentState>>,
) {
    let vault = repo_path.join(VAULT_DIR_NAME);
    if !vault.join(".git").is_file() {
        return;
    }
    let repo_id = format!(
        "{}-vault",
        repo_path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
    );
    let mut paths = Vec::new();
    walk_md_files(&vault, &mut paths);
    for file_path in &paths {
        ingest_file_chunks(&repo_id, file_path, pipeline, state).await;
    }
    let count = paths.len();
    let mut guard = state.lock().unwrap();
    guard.repos.insert(
        repo_id.clone(),
        RepoIndex { repo_id, chunk_count: count },
    );
}

pub async fn index_vaults_on_boot(state: Arc<Mutex<PrevalentState>>) {
    let vaults_dir = RepoWatcher::vaults_dir_from_env();
    let (tx, _rx) = std::sync::mpsc::channel();
    let watcher = RepoWatcher::new(vaults_dir, tx);
    let repos = watcher.scan_existing_repos();
    let mut pipeline = IndexingPipeline::new_from_env();
    for repo_path in &repos {
        index_repo(repo_path, &mut pipeline, &state).await;
        index_vault_worktree(repo_path, &mut pipeline, &state).await;
    }
    let mut guard = state.lock().unwrap();
    let _ = guard.search_index.commit();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn walker_excludes_prometheus_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let prometheus_dir = tmp.path().join("prometheus");
        fs::create_dir_all(&prometheus_dir).unwrap();
        fs::write(prometheus_dir.join("secret.md"), "# secret").unwrap();

        let mut results = Vec::new();
        walk_md_files(tmp.path(), &mut results);

        assert!(
            results.iter().all(|p| !p.starts_with(&prometheus_dir)),
            "prometheus/ files must not appear in walker results"
        );
    }

    #[test]
    fn walker_includes_normal_directories() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let docs_dir = tmp.path().join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        let md_file = docs_dir.join("guide.md");
        fs::write(&md_file, "# Guide").unwrap();

        let mut results = Vec::new();
        walk_md_files(tmp.path(), &mut results);

        assert!(
            results.contains(&md_file),
            "docs/guide.md must appear in walker results"
        );
    }
}
