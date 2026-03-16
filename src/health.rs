use axum::extract::State;
use axum::Json;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Serialize)]
pub struct LatencyPercentiles {
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub repos_warm: usize,
    pub repos_hibernated: usize,
    pub fila1_depth: usize,
    pub fila2_depth: usize,
    pub uptime_secs: u64,
    pub ollama_queue_depth: usize,
    pub embedding_debt_entries: usize,
    pub tracker_sync_queue_depth: usize,
    pub pks_query_latency_us: LatencyPercentiles,
    pub pks_ram_usage_bytes: u64,
}

pub async fn health_handler(
    State(state): State<Arc<Mutex<crate::state::PrevalentState>>>,
) -> Json<HealthResponse> {
    let guard = state.lock().unwrap();
    let repos_warm = guard.repos.len();
    let embedding_debt_entries = guard.embedding_debt.len();
    let uptime_secs = guard.started_at.elapsed().as_secs();
    drop(guard);
    Json(HealthResponse {
        status: "running",
        repos_warm,
        repos_hibernated: 0,
        fila1_depth: 0,
        fila2_depth: 0,
        uptime_secs,
        ollama_queue_depth: 0,
        embedding_debt_entries,
        tracker_sync_queue_depth: 0,
        pks_query_latency_us: LatencyPercentiles { p50: 0, p95: 0, p99: 0 },
        pks_ram_usage_bytes: 0,
    })
}
