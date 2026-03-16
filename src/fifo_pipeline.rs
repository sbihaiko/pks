use tokio::sync::mpsc;

use crate::debounce::Debouncer;
use crate::embedding_provider::{EmbeddingProvider, OllamaProvider};
use crate::indexer::pipeline::IndexingPipeline;
use crate::search::retriever::{SearchBackend, TantivyBackend};
use crate::state::{IndexMutation, PipelineEvent, RawTransaction};

fn fila1_capacity() -> usize {
    std::env::var("PKS_FILA1_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000)
}

fn fila2_capacity() -> usize {
    std::env::var("PKS_FILA2_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500)
}

async fn mutation_from_event(
    event: PipelineEvent,
    pipeline: &mut IndexingPipeline,
    embedder: &impl EmbeddingProvider,
) -> IndexMutation {
    match event {
        PipelineEvent::FileChanged { repo_id, file_path, content } => {
            let tagged_chunks = pipeline.process_file_with_dirty_markers(&repo_id, &file_path, &content);
            let mut results = Vec::new();
            for (chunk, is_dirty) in tagged_chunks {
                if is_dirty {
                    match embedder.embed_text(&chunk.text).await {
                        Ok(vec) => results.push((chunk, Some(vec))),
                        Err(_) => results.push((chunk, None)), // Degradation
                    }
                } else {
                    results.push((chunk, None)); // Reuse stored if exists or stay lazy
                }
            }
            IndexMutation::AddChunks(results)
        }
        PipelineEvent::FileDeleted { repo_id, file_path } => {
            IndexMutation::RemoveFile { repo_id, file_path }
        }
        PipelineEvent::RepoRegistered { .. } => IndexMutation::AddChunks(vec![]),
        PipelineEvent::RepoDeregistered { repo_id, .. } => {
            IndexMutation::RemoveFile { repo_id, file_path: String::new() }
        }
    }
}

async fn run_background_consumer(
    mut ingest_rx: mpsc::Receiver<RawTransaction>,
    mutations_tx: mpsc::Sender<IndexMutation>,
    mut pipeline: IndexingPipeline,
) {
    let mut debouncer = Debouncer::new_from_env();
    let embedder = OllamaProvider::from_env();
    while let Some(tx) = ingest_rx.recv().await {
        let key = Debouncer::make_dedup_key(
            tx.commit_sha.as_deref(),
            tx.tree_hash.as_deref(),
        );
        if debouncer.is_duplicate_and_record(&key) {
            continue;
        }
        let mutation = mutation_from_event(tx.event, &mut pipeline, &embedder).await;
        let _ = mutations_tx.try_send(mutation);
    }
}

pub struct FifoPipeline {
    ingest_tx: mpsc::Sender<RawTransaction>,
    mutations_rx: mpsc::Receiver<IndexMutation>,
}

impl FifoPipeline {
    pub fn new_and_spawn() -> Self {
        let (ingest_tx, ingest_rx) = mpsc::channel::<RawTransaction>(fila1_capacity());
        let (mutations_tx, mutations_rx) = mpsc::channel::<IndexMutation>(fila2_capacity());
        let pipeline = IndexingPipeline::new_from_env();
        tokio::spawn(run_background_consumer(ingest_rx, mutations_tx, pipeline));
        Self { ingest_tx, mutations_rx }
    }

    pub fn submit_transaction_to_ingest_queue(&self, tx: RawTransaction) {
        let _ = self.ingest_tx.try_send(tx);
    }

    pub fn drain_pending_mutations(&mut self, backend: &mut TantivyBackend) {
        let mut did_write = false;
        while let Ok(mutation) = self.mutations_rx.try_recv() {
            apply_mutation_to_backend(mutation, backend);
            did_write = true;
        }
        if did_write {
            let _ = backend.commit();
        }
    }
}

fn apply_mutation_to_backend(mutation: IndexMutation, backend: &mut TantivyBackend) {
    match mutation {
        IndexMutation::AddChunks(chunks_with_vecs) => {
            for (chunk, vector) in chunks_with_vecs {
                if chunk.is_tombstone {
                    let _ = backend.remove_chunks_for_file(&chunk.repo_id, &chunk.file_path);
                    continue;
                }
                if let Some(v) = vector {
                    let _ = backend.add_chunk_with_vector(&chunk, v);
                } else {
                    let _ = backend.add_chunk(&chunk);
                }
            }
        }
        IndexMutation::RemoveFile { repo_id, file_path } => {
            if !file_path.is_empty() {
                let _ = backend.remove_chunks_for_file(&repo_id, &file_path);
            }
        }
    }
}
