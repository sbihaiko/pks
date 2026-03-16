use dotenvy::dotenv;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use pks::indexer::pipeline::IndexingPipeline;
use pks::mcp_server::McpServer;
use pks::repo_watcher::RepoWatcher;
use pks::search::retriever::SearchBackend;
use pks::state::{PrevalentState, RepoIndex};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let _log_guard = pks::observability::init_logging(&pks::observability::log_config_from_env());

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && !args[1].starts_with('-') {
        let cmd = pks::cli::parse_args(&args);
        let exit_code = pks::cli::run_command(cmd).await;
        std::process::exit(exit_code);
    }

    if args.contains(&"--stdio".to_string()) {
        run_stdio_server().await;
        return;
    }

    let state = Arc::new(Mutex::new(PrevalentState::default()));
    let port = McpServer::port_from_env();
    let server = McpServer::new(port);
    let ct = server.cancellation_token();

    index_vaults_on_boot(Arc::clone(&state)).await;

    let state_for_shutdown = Arc::clone(&state);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutdown signal received (Ctrl+C)...");
        ct.cancel();
    });

    #[cfg(unix)]
    let ct_terminate = server.cancellation_token();
    #[cfg(unix)]
    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        sigterm.recv().await;
        tracing::info!("Shutdown signal received (SIGTERM)...");
        ct_terminate.cancel();
    });

    tracing::info!("PKS Daemon starting on port {port}...");

    let run_handle = tokio::spawn(server.run(Arc::clone(&state)));

    // Wait for server or signals
    let _ = run_handle.await;

    tracing::info!("Saving snapshots before shutdown...");
    let guard = state_for_shutdown.lock().unwrap();
    if let Err(e) = guard.save_all_snapshots() {
        tracing::error!("Failed to save snapshots during shutdown: {e}");
    } else {
        tracing::info!("Snapshots saved successfully.");
    }
}

fn collect_md_entry(path: PathBuf, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        walk_md_files(&path, out);
        return;
    }
    if path.extension().map_or(false, |ext| ext == "md") {
        out.push(path);
    }
}

fn walk_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return; };
    for entry in entries.filter_map(|e| e.ok()) {
        collect_md_entry(entry.path(), out);
    }
}

async fn ingest_file_chunks(
    repo_id: &str,
    file_path: &Path,
    pipeline: &mut IndexingPipeline,
    state: &Arc<Mutex<PrevalentState>>,
) {
    use pks::embedding_provider::{EmbeddingProvider, OllamaProvider};
    let Ok(content) = std::fs::read_to_string(file_path) else { return; };
    let file_str = file_path.to_string_lossy().into_owned();
    let tagged_chunks = pipeline.process_file_with_dirty_markers(repo_id, &file_str, &content);
    let embedder = OllamaProvider::from_env();

    let mut results = Vec::new();
    for (chunk, is_dirty) in tagged_chunks {
        if is_dirty {
            let vec = embedder.embed_text(&chunk.text).await.ok();
            results.push((chunk, vec));
        } else {
            results.push((chunk, None));
        }
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

async fn index_repo(
    repo_path: &Path,
    pipeline: &mut IndexingPipeline,
    state: &Arc<Mutex<PrevalentState>>,
) {
    let repo_id = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut markdown_file_paths = Vec::new();
    walk_md_files(repo_path, &mut markdown_file_paths);
    for file_path in &markdown_file_paths {
        ingest_file_chunks(&repo_id, file_path, pipeline, state).await;
    }
    let mut guard = state.lock().unwrap();
    guard.repos.insert(repo_id.clone(), RepoIndex { repo_id, chunk_count: markdown_file_paths.len() });
}

async fn run_stdio_server() {
    use rmcp::ServiceExt;
    let state = Arc::new(Mutex::new(PrevalentState::default()));
    let state_for_indexing = Arc::clone(&state);
    tokio::spawn(async move {
        index_vaults_on_boot(state_for_indexing).await;
    });
    let handler = pks::mcp_server::PksHandler::new(state);
    let transport = rmcp::transport::io::stdio();
    match handler.serve(transport).await {
        Err(e) => eprintln!("stdio init error: {e}"),
        Ok(server) => {
            let _ = server.waiting().await;
        }
    }
}

async fn index_vaults_on_boot(state: Arc<Mutex<PrevalentState>>) {
    let vaults_dir = RepoWatcher::vaults_dir_from_env();
    let (tx, _rx) = std::sync::mpsc::channel();
    let watcher = RepoWatcher::new(vaults_dir, tx);
    let repos = watcher.scan_existing_repos();
    let mut pipeline = IndexingPipeline::new_from_env();

    for repo_path in &repos {
        index_repo(repo_path, &mut pipeline, &state).await;
    }

    let mut guard = state.lock().unwrap();
    let _ = guard.search_index.commit();
}
