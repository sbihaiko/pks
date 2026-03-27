use std::collections::HashMap;

use super::retriever::{ChunkMeta, SearchResult};

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    let denom = norm_a * norm_b;
    if denom < f32::EPSILON {
        return 0.0;
    }
    dot / denom
}

fn accumulate_bm25_rrf_scores(
    bm25_results: &[SearchResult],
    rrf_k: f32,
    scores: &mut HashMap<String, f32>,
) {
    for (rank, result) in bm25_results.iter().enumerate() {
        let entry = scores.entry(result.chunk_hash.clone()).or_insert(0.0);
        *entry += 1.0 / (rrf_k + rank as f32 + 1.0);
    }
}

fn accumulate_sorted_vector_rrf_scores(
    vector_results: &[(String, f32)],
    rrf_k: f32,
    scores: &mut HashMap<String, f32>,
) {
    let mut sorted = vector_results.to_vec();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (rank, (chunk_hash, _)) in sorted.iter().enumerate() {
        let entry = scores.entry(chunk_hash.clone()).or_insert(0.0);
        *entry += 1.0 / (rrf_k + rank as f32 + 1.0);
    }
}

fn build_bm25_lookup(bm25_results: &[SearchResult]) -> HashMap<String, &SearchResult> {
    bm25_results.iter().map(|r| (r.chunk_hash.clone(), r)).collect()
}

fn resolve_metadata<'a>(
    chunk_hash: &str,
    bm25_lookup: &HashMap<String, &'a SearchResult>,
    vector_meta: &'a HashMap<String, ChunkMeta>,
) -> (String, String, Vec<String>, String) {
    if let Some(r) = bm25_lookup.get(chunk_hash) {
        return (r.chunk_text.clone(), r.file_path.clone(), r.heading_hierarchy.clone(), r.repo_id.clone());
    }
    if let Some(m) = vector_meta.get(chunk_hash) {
        return (m.text.clone(), m.file_path.clone(), m.heading_hierarchy.clone(), m.repo_id.clone());
    }
    (String::new(), String::new(), vec![], String::new())
}

fn build_ranked_rrf_results(
    bm25_results: &[SearchResult],
    scores: HashMap<String, f32>,
    top_k: usize,
    vector_meta: &HashMap<String, ChunkMeta>,
) -> Vec<SearchResult> {
    let lookup = build_bm25_lookup(bm25_results);
    let mut merged: Vec<SearchResult> = scores
        .iter()
        .filter_map(|(chunk_hash, &score)| {
            let (chunk_text, file_path, heading_hierarchy, repo_id) =
                resolve_metadata(chunk_hash, &lookup, vector_meta);
            if repo_id.is_empty() { return None; }
            Some(SearchResult { file_path, heading_hierarchy, chunk_text, chunk_hash: chunk_hash.clone(), score, repo_id })
        })
        .collect();
    merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    merged.dedup_by(|a, b| a.chunk_hash == b.chunk_hash);
    merged.truncate(top_k);
    merged
}

pub fn reciprocal_rank_fusion(
    bm25_results: &[SearchResult],
    vector_results: &[(String, f32)],
    top_k: usize,
    vector_meta: &HashMap<String, ChunkMeta>,
) -> Vec<SearchResult> {
    const RRF_K: f32 = 60.0;
    let mut scores: HashMap<String, f32> = HashMap::new();
    accumulate_bm25_rrf_scores(bm25_results, RRF_K, &mut scores);
    accumulate_sorted_vector_rrf_scores(vector_results, RRF_K, &mut scores);
    build_ranked_rrf_results(bm25_results, scores, top_k, vector_meta)
}

pub fn search_hybrid(
    stored_vectors: &HashMap<String, Vec<f32>>,
    query_vector: &[f32],
    bm25_results: Vec<SearchResult>,
    top_k: usize,
    vector_meta: &HashMap<String, ChunkMeta>,
) -> Vec<SearchResult> {
    if stored_vectors.is_empty() {
        return bm25_results.into_iter().take(top_k).collect();
    }

    let vector_results: Vec<(String, f32)> = stored_vectors
        .iter()
        .map(|(chunk_hash, vec)| {
            let similarity = cosine_similarity(query_vector, vec);
            (chunk_hash.clone(), similarity)
        })
        .collect();

    reciprocal_rank_fusion(&bm25_results, &vector_results, top_k, vector_meta)
}
