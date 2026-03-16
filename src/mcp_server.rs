use crate::health::health_handler;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

pub const DEFAULT_PORT: u16 = 3030;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub file_path: String,
    pub heading_hierarchy: Vec<String>,
    pub chunk_text: String,
    pub score: f32,
    pub repo_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchKnowledgeVaultParams {
    pub query: String,
    pub top_k: Option<u32>,
    pub projects_filter: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct PksHandler {
    tool_router: ToolRouter<Self>,
    state: Arc<std::sync::Mutex<crate::state::PrevalentState>>,
}

#[tool_router]
impl PksHandler {
    #[tool(
        name = "search_knowledge_vault",
        description = "Search indexed knowledge vaults using Hybrid (BM25 + Semantic) search."
    )]
    async fn search_knowledge_vault(
        &self,
        Parameters(params): Parameters<SearchKnowledgeVaultParams>,
    ) -> String {
        use crate::search::retriever::SearchBackend;
        use crate::embedding_provider::{EmbeddingProvider, OllamaProvider};
        use crate::search::hybrid::search_hybrid;

        let top_k = params.top_k.unwrap_or(10) as usize;
        let filter: Option<Vec<String>> = params.projects_filter.clone();
        let projects_filter_slice: Option<&[String]> = filter.as_deref();

        let embedder = OllamaProvider::from_env();
        let query_vector = embedder.embed_text(&params.query).await.unwrap_or_default();

        let guard = self.state.lock().unwrap();
        let bm25_results_raw = guard.search_index.search(&params.query, top_k * 2, projects_filter_slice)
            .unwrap_or_default();

        let bm25_only: Vec<_> = bm25_results_raw.into_iter().take(top_k).collect();
        let results_raw = query_vector.is_empty().then(|| bm25_only.clone())
            .unwrap_or_else(|| search_hybrid(&guard.search_index.vectors, &query_vector, bm25_only, top_k));

        let results: Vec<SearchResult> = results_raw.into_iter().map(|r| SearchResult {
            file_path: r.file_path,
            heading_hierarchy: r.heading_hierarchy,
            chunk_text: r.chunk_text,
            score: r.score,
            repo_id: r.repo_id,
        }).collect();
        serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
    }

    #[tool(
        name = "list_knowledge_vaults",
        description = "List all registered Git repository vaults known to PKS."
    )]
    async fn list_knowledge_vaults(&self) -> String {
        let guard = self.state.lock().unwrap();
        let ids = guard.list_repo_ids();
        serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string())
    }
}

#[tool_handler]
impl ServerHandler for PksHandler {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }
}

impl PksHandler {
    pub fn new(state: Arc<std::sync::Mutex<crate::state::PrevalentState>>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            state,
        }
    }
}

pub struct McpServer {
    addr: SocketAddr,
    cancellation_token: CancellationToken,
}

impl McpServer {
    pub fn new(port: u16) -> Self {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        Self {
            addr,
            cancellation_token: CancellationToken::new(),
        }
    }

    pub fn port_from_env() -> u16 {
        env::var("PKS_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_PORT)
    }

    pub fn bind_addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn run(
        self,
        state: Arc<Mutex<crate::state::PrevalentState>>,
    ) -> std::io::Result<()> {
        use axum::Router;
        use axum::routing::get;
        use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
        use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

        let ct = self.cancellation_token.clone();
        let config = StreamableHttpServerConfig { cancellation_token: ct.child_token(), ..Default::default() };
        let health_state = Arc::clone(&state);
        let service: StreamableHttpService<PksHandler, LocalSessionManager> = StreamableHttpService::new(
            move || Ok(PksHandler::new(Arc::clone(&state))),
            Arc::new(LocalSessionManager::default()),
            config,
        );
        let router = Router::new()
            .nest_service("/sse", service)
            .route("/health", get(health_handler))
            .with_state(health_state);
        let listener = TcpListener::bind(self.addr).await?;
        axum::serve(listener, router)
            .with_graceful_shutdown(async move { ct.cancelled_owned().await })
            .await
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }
}
