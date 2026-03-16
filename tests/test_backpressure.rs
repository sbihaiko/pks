use pks::fifo_pipeline::FifoPipeline;
use pks::search::retriever::{SearchBackend, TantivyBackend};
use pks::state::{PipelineEvent, RawTransaction};
use std::sync::Mutex;

// Serialize env var access across the test process to avoid races with parallel tests.
static OLLAMA_ENV_LOCK: Mutex<()> = Mutex::new(());

fn fila1_max() -> usize {
    std::env::var("PKS_FILA1_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000)
}

fn make_file_changed_transaction(index: usize) -> RawTransaction {
    let content = format!(
        "# Section {index}\nRust async pipeline backpressure document number {index}. \
        This content tests BM25 indexing under load conditions."
    );
    RawTransaction {
        event: PipelineEvent::FileChanged {
            repo_id: "bp_repo".to_string(),
            file_path: format!("doc_{index}.md"),
            content,
        },
        commit_sha: Some(format!("sha_{index:08x}")),
        tree_hash: None,
        branch: Some("main".to_string()),
        ingested_at: std::time::Instant::now(),
    }
}

#[tokio::test]
async fn queries_stay_fast_while_queue1_is_saturated() {
    // Route Ollama to a definitely-closed port so embed calls fail immediately (ECONNREFUSED).
    // This is not a mock — it configures a real provider to use an unavailable endpoint,
    // matching the 5-state degradation spec (Ollama not running → BM25-only).
    let _env_guard = OLLAMA_ENV_LOCK.lock().unwrap();
    std::env::set_var("OLLAMA_BASE_URL", "http://127.0.0.1:19997");

    let backend = TantivyBackend::new_in_memory().unwrap();
    let pipeline = FifoPipeline::new_and_spawn();

    let overflow_count = fila1_max() + 200;
    for i in 0..overflow_count {
        pipeline.submit_transaction_to_ingest_queue(make_file_changed_transaction(i));
    }

    let query_start = std::time::Instant::now();
    let results = backend.search("backpressure pipeline", 5, None).unwrap();
    let query_elapsed = query_start.elapsed();

    let threshold_ms: u128 = if cfg!(debug_assertions) { 50 } else { 5 };
    assert!(
        query_elapsed.as_millis() < threshold_ms,
        "query must complete in < {threshold_ms}ms while queue is saturated, took {}ms",
        query_elapsed.as_millis()
    );

    drop(results);
    drop(pipeline);

    let mut backend2 = TantivyBackend::new_in_memory().unwrap();
    let mut pipeline2 = FifoPipeline::new_and_spawn();

    for i in 0..10 {
        pipeline2.submit_transaction_to_ingest_queue(make_file_changed_transaction(i));
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    pipeline2.drain_pending_mutations(&mut backend2);

    let indexed = backend2.search("backpressure pipeline", 10, None).unwrap();
    assert!(
        !indexed.is_empty(),
        "after drain_pending_mutations, backend must have indexed at least one chunk"
    );

    std::env::remove_var("OLLAMA_BASE_URL");
}
