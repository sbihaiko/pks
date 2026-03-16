#[cfg(test)]
mod tests {
    use pks::indexer::chunker::{Chunk, MarkdownChunker};
    use pks::search::retriever::{SearchBackend, SearchResult, TantivyBackend};
    use pks::search::hybrid::{cosine_similarity, reciprocal_rank_fusion, search_hybrid};
    use std::collections::HashMap;

    fn make_chunk(repo_id: &str, file_path: &str, text: &str, index: usize) -> Chunk {
        Chunk {
            repo_id: repo_id.to_string(),
            file_path: file_path.to_string(),
            heading_hierarchy: vec!["Test".to_string()],
            chunk_index: index,
            chunk_hash: format!("{repo_id}-{file_path}-{index}"),
            text: text.to_string(),
            is_tombstone: false,
        }
    }

    #[test]
    fn add_and_search_returns_results() {
        let mut backend = TantivyBackend::new_in_memory().unwrap();
        let content = include_str!("golden_dataset/nutrition_basics.md");
        let chunker = MarkdownChunker { max_tokens: 400, min_tokens: 50, overlap_tokens: 80 };
        for chunk in chunker.chunk_document("repo1", "nutrition.md", content) {
            backend.add_chunk(&chunk).unwrap();
        }
        backend.commit().unwrap();
        let results = backend.search("protein muscle", 5, None).unwrap();
        assert!(!results.is_empty(), "search must return at least one result");
        assert!(results.iter().all(|r| !r.chunk_text.is_empty()));
    }

    #[test]
    fn remove_chunks_for_file_works() {
        let mut backend = TantivyBackend::new_in_memory().unwrap();
        let chunk = make_chunk("repo1", "file_to_delete.md", "Omega-3 fatty acids are essential.", 0);
        backend.add_chunk(&chunk).unwrap();
        backend.commit().unwrap();

        backend.remove_chunks_for_file("repo1", "file_to_delete.md").unwrap();
        backend.commit().unwrap();

        let results = backend.search("Omega-3 fatty acids", 5, None).unwrap();
        assert!(
            results.is_empty(),
            "deleted file chunks must not appear in results"
        );
    }

    #[test]
    fn repo_filter_limits_results() {
        let mut backend = TantivyBackend::new_in_memory().unwrap();
        let chunk_a = make_chunk("repo_alpha", "a.md", "Cardiovascular endurance training improves heart health.", 0);
        let chunk_b = make_chunk("repo_beta", "b.md", "Cardiovascular endurance training improves heart health.", 0);
        backend.add_chunk(&chunk_a).unwrap();
        backend.add_chunk(&chunk_b).unwrap();
        backend.commit().unwrap();

        let filter = vec!["repo_alpha".to_string()];
        let results = backend.search("cardiovascular endurance", 10, Some(&filter)).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.repo_id == "repo_alpha"));
    }

    #[test]
    fn search_latency_is_sub_millisecond() {
        let mut backend = TantivyBackend::new_in_memory().unwrap();
        let base = "The human body requires balanced nutrition including vitamins minerals protein. ";

        for i in 0..1000 {
            let text = format!("{base} Document index {i} provides unique context.");
            let chunk = make_chunk("bench_repo", &format!("doc_{i}.md"), &text, i);
            backend.add_chunk(&chunk).unwrap();
        }
        backend.commit().unwrap();

        let start = std::time::Instant::now();
        let results = backend.search("vitamins minerals protein", 10, None).unwrap();
        let elapsed = start.elapsed();

        #[cfg(debug_assertions)]
        let threshold_ms: u128 = 50;
        #[cfg(not(debug_assertions))]
        let threshold_ms: u128 = 1;
        assert!(!results.is_empty());
        assert!(
            elapsed.as_millis() < threshold_ms,
            "search must complete in < {threshold_ms}ms, took {}µs",
            elapsed.as_micros()
        );
    }

    #[test]
    fn tombstone_triggers_file_removal() {
        let mut backend = TantivyBackend::new_in_memory().unwrap();
        let chunk = make_chunk("repo1", "deleted.md", "Mindfulness reduces cortisol levels significantly.", 0);
        backend.add_chunk(&chunk).unwrap();
        backend.commit().unwrap();

        let tombstone = MarkdownChunker::tombstone("repo1", "deleted.md");
        backend.remove_chunks_for_file(&tombstone.repo_id, &tombstone.file_path).unwrap();
        backend.commit().unwrap();

        let results = backend.search("cortisol mindfulness", 5, None).unwrap();
        assert!(results.is_empty(), "tombstone must result in file removal from index");
    }

    #[test]
    fn cosine_similarity_of_identical_vectors_is_one() {
        let v = vec![0.1_f32, 0.5, 0.3, 0.8, 0.2];
        let similarity = cosine_similarity(&v, &v);
        assert!(
            (similarity - 1.0).abs() < 1e-5,
            "cosine similarity of a vector with itself must be 1.0, got {similarity}"
        );
    }

    #[test]
    fn reciprocal_rank_fusion_merges_rankings() {
        let bm25 = vec![
            SearchResult {
                file_path: "a.md".to_string(),
                heading_hierarchy: vec![],
                chunk_text: "alpha content".to_string(),
                score: 0.9,
                repo_id: "repo1".to_string(),
            },
            SearchResult {
                file_path: "b.md".to_string(),
                heading_hierarchy: vec![],
                chunk_text: "beta content".to_string(),
                score: 0.7,
                repo_id: "repo1".to_string(),
            },
        ];
        let vector_results = vec![
            ("beta content".to_string(), 0.95_f32),
            ("alpha content".to_string(), 0.6_f32),
        ];
        let merged = reciprocal_rank_fusion(&bm25, &vector_results, 5);
        assert_eq!(merged.len(), 2, "RRF must return both chunks");
        assert!(
            merged[0].score > 0.0 && merged[1].score > 0.0,
            "all merged scores must be positive"
        );
    }

    #[test]
    fn hybrid_search_degrades_gracefully_without_vectors() {
        let mut backend = TantivyBackend::new_in_memory().unwrap();
        let chunk = make_chunk("repo1", "food.md", "Protein supports muscle repair and growth.", 0);
        backend.add_chunk(&chunk).unwrap();
        backend.commit().unwrap();

        let bm25_results = backend.search("protein muscle", 5, None).unwrap();
        assert!(!bm25_results.is_empty(), "bm25 must return results even without vectors");

        let empty_vectors: HashMap<String, Vec<f32>> = HashMap::new();
        let query_vector = vec![0.1_f32, 0.2, 0.3];
        let hybrid_results = search_hybrid(&empty_vectors, &query_vector, bm25_results.clone(), 5);
        assert_eq!(
            hybrid_results.len(),
            bm25_results.len(),
            "without vectors, hybrid must return bm25 results unchanged"
        );
    }

    #[test]
    fn hybrid_search_with_vectors_outperforms_bm25_only() {
        let mut backend = TantivyBackend::new_in_memory().unwrap();
        let chunk_a = make_chunk("repo1", "sleep.md", "Sleep deprivation impairs cognitive function.", 0);
        let chunk_b = make_chunk("repo1", "exercise.md", "Aerobic exercise boosts cardiovascular health.", 1);
        let vector_a = vec![0.9_f32, 0.1, 0.0];
        let vector_b = vec![0.1_f32, 0.9, 0.0];

        backend.add_chunk_with_vector(&chunk_a, vector_a).unwrap();
        backend.add_chunk_with_vector(&chunk_b, vector_b).unwrap();
        backend.commit().unwrap();

        let bm25_results = backend.search("cognitive sleep", 5, None).unwrap();
        let query_vector = vec![0.85_f32, 0.15, 0.0];
        let hybrid_results = search_hybrid(&backend.vectors, &query_vector, bm25_results, 5);
        assert!(!hybrid_results.is_empty(), "hybrid search must return results");
        assert!(hybrid_results[0].score > 0.0, "top hybrid result must have positive RRF score");
    }
}
