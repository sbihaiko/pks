use std::sync::{Arc, Mutex};

use crate::ipc::{IPC_VER, PksCommand, PksResponse, SearchHit, SOCKET_PATH};
use crate::search::retriever::SearchBackend;
use crate::state::PrevalentState;

/// Server — listens on the Unix Domain Socket and dispatches commands.
pub struct IpcServer {
    pub(crate) state: Arc<Mutex<PrevalentState>>,
    socket_path: String,
}

impl IpcServer {
    pub fn new(state: Arc<Mutex<PrevalentState>>) -> Self {
        Self { state, socket_path: SOCKET_PATH.to_string() }
    }

    /// Create a server with a custom socket path (useful for testing)
    pub fn with_socket_path(state: Arc<Mutex<PrevalentState>>, path: impl Into<String>) -> Self {
        Self { state, socket_path: path.into() }
    }

    #[cfg(unix)]
    pub async fn accept_loop(self: Arc<Self>) {
        use tokio::net::UnixListener;
        let _ = std::fs::remove_file(&self.socket_path);
        let listener = match UnixListener::bind(&self.socket_path) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("IpcServer bind error on {}: {e}", self.socket_path);
                return;
            }
        };
        tracing::info!("IpcServer listening on {}", self.socket_path);
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let server = Arc::clone(&self);
                    tokio::spawn(async move { server.handle_connection(stream).await });
                }
                Err(e) => tracing::warn!("accept error: {e}"),
            }
        }
    }

    #[cfg(unix)]
    pub(crate) async fn handle_connection(&self, stream: tokio::net::UnixStream) {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let (reader, mut writer) = tokio::io::split(stream);
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        if reader.read_line(&mut line).await.is_err() {
            return;
        }
        let response = match serde_json::from_str::<PksCommand>(line.trim()) {
            Err(e) => PksResponse::Err { message: format!("parse error: {e}") },
            Ok(cmd) => self.dispatch(cmd),
        };
        if let Ok(mut resp_json) = serde_json::to_string(&response) {
            resp_json.push('\n');
            let _ = writer.write_all(resp_json.as_bytes()).await;
        }
    }

    pub(crate) fn dispatch(&self, cmd: PksCommand) -> PksResponse {
        match cmd {
            PksCommand::Ping => PksResponse::Pong { ver: IPC_VER },
            PksCommand::ListVaults => {
                match self.state.lock() {
                    Ok(guard) => PksResponse::VaultList { vaults: guard.list_repo_ids() },
                    Err(_) => PksResponse::Err { message: "state lock poisoned".to_string() },
                }
            }
            PksCommand::Search { query, top_n, .. } => self.dispatch_search(&query, top_n),
            PksCommand::Refresh { dry_run } => self.dispatch_refresh(dry_run),
            PksCommand::RecordToolEvent { .. } => {
                // Shadow journaling deferred to M12 — acknowledge without processing
                PksResponse::EventRecorded
            }
        }
    }

    fn dispatch_refresh(&self, dry_run: bool) -> PksResponse {
        use crate::repo_watcher::RepoWatcher;
        let vaults_dir = RepoWatcher::vaults_dir_from_env();
        let (tx, _rx) = std::sync::mpsc::channel();
        let watcher = RepoWatcher::new(vaults_dir, tx);
        let found: Vec<String> = watcher.scan_existing_repos()
            .into_iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        let registered = match self.state.lock() {
            Ok(g) => g.list_repo_ids(),
            Err(_) => return PksResponse::Err { message: "state lock poisoned".to_string() },
        };
        let added: Vec<String> = found.iter()
            .filter(|n| !registered.contains(n)).cloned().collect();
        let removed: Vec<String> = registered.iter()
            .filter(|n| !found.contains(n)).cloned().collect();
        let unchanged: Vec<String> = found.iter()
            .filter(|n| registered.contains(n)).cloned().collect();
        if !dry_run {
            tracing::info!("pks refresh: +{} -{} ={}", added.len(), removed.len(), unchanged.len());
        }
        PksResponse::RefreshDone { added, removed, unchanged }
    }

    fn dispatch_search(&self, query: &str, top_n: usize) -> PksResponse {
        let guard = match self.state.lock() {
            Ok(g) => g,
            Err(_) => return PksResponse::Err { message: "state lock poisoned".to_string() },
        };
        let results = guard.search_index.search(query, top_n, None).unwrap_or_default();
        let hits = results
            .into_iter()
            .map(|r| SearchHit {
                repo_id: r.repo_id,
                file_path: r.file_path,
                score: r.score,
                snippet: r.chunk_text,
            })
            .collect();
        PksResponse::SearchResults { hits }
    }
}
