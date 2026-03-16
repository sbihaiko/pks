use std::collections::HashMap;

use super::retriever::SearchResult;

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
        let entry = scores.entry(result.chunk_text.clone()).or_insert(0.0);
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
    for (rank, (chunk_text, _)) in sorted.iter().enumerate() {
        let entry = scores.entry(chunk_text.clone()).or_insert(0.0);
        *entry += 1.0 / (rrf_k + rank as f32 + 1.0);
    }
}

fn build_bm25_lookup(bm25_results: &[SearchResult]) -> HashMap<String, &SearchResult> {
    bm25_results.iter().map(|r| (r.chunk_text.clone(), r)).collect()
}

fn build_ranked_rrf_results(
    bm25_results: &[SearchResult],
    scores: HashMap<String, f32>,
    top_k: usize,
) -> Vec<SearchResult> {
    let lookup = build_bm25_lookup(bm25_results);
    let mut merged: Vec<SearchResult> = scores
        .iter()
        .map(|(chunk_text, &score)| {
            let (file_path, heading_hierarchy, repo_id) = match lookup.get(chunk_text) {
                Some(r) => (r.file_path.clone(), r.heading_hierarchy.clone(), r.repo_id.clone()),
                None => (String::new(), vec![], String::new()),
            };
            SearchResult { file_path, heading_hierarchy, chunk_text: chunk_text.clone(), score, repo_id }
        })
        .collect();
    merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    merged.dedup_by(|a, b| a.chunk_text == b.chunk_text);
    merged.truncate(top_k);
    merged
}

pub fn reciprocal_rank_fusion(
    bm25_results: &[SearchResult],
    vector_results: &[(String, f32)],
    top_k: usize,
) -> Vec<SearchResult> {
    const RRF_K: f32 = 60.0;
    let mut scores: HashMap<String, f32> = HashMap::new();
    accumulate_bm25_rrf_scores(bm25_results, RRF_K, &mut scores);
    accumulate_sorted_vector_rrf_scores(vector_results, RRF_K, &mut scores);
    build_ranked_rrf_results(bm25_results, scores, top_k)
}

pub fn search_hybrid(
    stored_vectors: &HashMap<String, Vec<f32>>,
    query_vector: &[f32],
    bm25_results: Vec<SearchResult>,
    top_k: usize,
) -> Vec<SearchResult> {
    if stored_vectors.is_empty() {
        return bm25_results.into_iter().take(top_k).collect();
    }

    let vector_results: Vec<(String, f32)> = stored_vectors
        .iter()
        .map(|(chunk_text, vec)| {
            let similarity = cosine_similarity(query_vector, vec);
            (chunk_text.clone(), similarity)
        })
        .collect();

    reciprocal_rank_fusion(&bm25_results, &vector_results, top_k)
}
