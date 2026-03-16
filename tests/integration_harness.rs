use pks::indexer::chunker::MarkdownChunker;
use pks::search::retriever::{SearchBackend, SearchResult, TantivyBackend};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[cfg(feature = "integration")]
#[derive(Deserialize)]
struct GoldenQuery {
    query: String,
    expected_top3: Vec<String>,
    score_floor: f32,
}

#[cfg(feature = "integration")]
#[derive(Deserialize)]
struct GoldenQueryFile {
    queries: Vec<GoldenQuery>,
}

#[cfg(feature = "integration")]
fn load_golden_dataset(dir: &Path) -> Vec<(String, String)> {
    let entries = fs::read_dir(dir).expect("golden_dataset dir must exist");
    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x == "md")
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let path = e.path();
            let stem = path.file_stem()?.to_str()?.to_string();
            let content = fs::read_to_string(&path).ok()?;
            Some((stem, content))
        })
        .collect()
}

#[cfg(feature = "integration")]
fn build_index(dataset: &[(String, String)]) -> TantivyBackend {
    let chunker = MarkdownChunker {
        max_tokens: 400,
        min_tokens: 100,
        overlap_tokens: 80,
    };
    let mut backend = TantivyBackend::new_in_memory().expect("in-memory backend must init");
    for (stem, content) in dataset {
        let file_path = format!("{}.md", stem);
        let chunks = chunker.chunk_document("golden", &file_path, content);
        for chunk in &chunks {
            backend.add_chunk(chunk).expect("add_chunk must succeed");
        }
    }
    backend.commit().expect("commit must succeed");
    backend
}

#[cfg(feature = "integration")]
fn load_queries(path: &Path) -> Vec<GoldenQuery> {
    let raw = fs::read_to_string(path).expect("golden_queries.toml must be readable");
    let parsed: GoldenQueryFile = toml::from_str(&raw).expect("golden_queries.toml must parse");
    parsed.queries
}

#[cfg(feature = "integration")]
fn result_file_stems(results: &[SearchResult]) -> Vec<String> {
    results
        .iter()
        .filter_map(|r| {
            Path::new(&r.file_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .collect()
}

#[cfg(feature = "integration")]
fn query_passes(gq: &GoldenQuery, results: &[SearchResult]) -> bool {
    let top_score = results.first().map(|r| r.score).unwrap_or(0.0);
    let stems = result_file_stems(results);
    let score_ok = top_score >= gq.score_floor;
    let top3_match = gq
        .expected_top3
        .iter()
        .any(|expected| stems.iter().any(|s| s.contains(expected.as_str())));
    score_ok && top3_match
}

#[cfg(feature = "integration")]
#[test]
fn full_pipeline_golden_dataset_passes_score_floors() {
    let dataset_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden_dataset");
    let queries_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden_queries.toml");

    let dataset = load_golden_dataset(&dataset_dir);
    assert!(
        dataset.len() >= 50,
        "golden dataset must have at least 50 notes, found {}",
        dataset.len()
    );

    let backend = build_index(&dataset);
    let queries = load_queries(&queries_path);
    assert!(
        queries.len() >= 20,
        "golden_queries.toml must have at least 20 queries, found {}",
        queries.len()
    );

    let passed = queries
        .iter()
        .filter(|gq| {
            let results = backend.search(&gq.query, 3, None).unwrap_or_default();
            query_passes(gq, &results)
        })
        .count();

    let total = queries.len();
    let accuracy = passed as f32 / total as f32;

    assert!(
        accuracy >= 0.90,
        "Golden dataset accuracy {:.1}% below 90% threshold ({}/{} queries passed)",
        accuracy * 100.0,
        passed,
        total
    );
}
