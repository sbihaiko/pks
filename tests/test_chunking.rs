#[cfg(test)]
mod tests {
    use pks::indexer::chunker::MarkdownChunker;

    fn make_chunker() -> MarkdownChunker {
        MarkdownChunker { max_tokens: 400, min_tokens: 100, overlap_tokens: 80 }
    }

    fn long_section_text() -> String {
        let sentence = "The quick brown fox jumps over the lazy dog and then rests. ";
        sentence.repeat(60)
    }

    #[test]
    fn heading_split_produces_multiple_chunks() {
        let content = include_str!("golden_dataset/nutrition_basics.md");
        let chunker = make_chunker();
        let chunks = chunker.chunk_document("repo1", "nutrition.md", content);
        assert!(chunks.len() > 1, "expected multiple chunks from multi-heading doc");
        assert!(chunks.iter().all(|c| !c.is_tombstone));
    }

    #[test]
    fn small_sections_are_merged() {
        let content = "## Alpha\n\nTiny.\n\n## Beta\n\nAlso tiny.\n\n## Gamma\n\nAnd this too.\n";
        let chunker = make_chunker();
        let chunks = chunker.chunk_document("repo1", "small.md", content);
        assert!(
            chunks.len() <= 2,
            "small sections should be merged, got {} chunks",
            chunks.len()
        );
    }

    #[test]
    fn long_section_applies_sliding_window() {
        let text = long_section_text();
        let content = format!("## Long Section\n\n{}\n", text);
        let chunker = make_chunker();
        let chunks = chunker.chunk_document("repo1", "long.md", &content);
        assert!(
            chunks.len() > 1,
            "long section must be split into multiple chunks via sliding window"
        );
    }

    #[test]
    fn tombstone_has_is_tombstone_true() {
        let t = MarkdownChunker::tombstone("repo1", "some/file.md");
        assert!(t.is_tombstone);
        assert_eq!(t.repo_id, "repo1");
        assert_eq!(t.file_path, "some/file.md");
        assert!(t.text.is_empty());
    }

    #[test]
    fn chunk_hash_is_consistent() {
        let content = include_str!("golden_dataset/sleep_hygiene.md");
        let chunker = make_chunker();
        let first = chunker.chunk_document("repo1", "sleep.md", content);
        let second = chunker.chunk_document("repo1", "sleep.md", content);
        let hashes_a: Vec<&str> = first.iter().map(|c| c.chunk_hash.as_str()).collect();
        let hashes_b: Vec<&str> = second.iter().map(|c| c.chunk_hash.as_str()).collect();
        assert_eq!(hashes_a, hashes_b, "hashes must be deterministic");
    }

    fn deep_section_text() -> String {
        let sentence = "Deep level content provides important context about the nested topic at hand. ";
        sentence.repeat(10)
    }

    #[test]
    fn heading_hierarchy_is_preserved() {
        let deep_text = deep_section_text();
        let content = format!(
            "# Top\n\n## Sub\n\n### Deep\n\n{deep_text}\n"
        );
        let chunker = make_chunker();
        let chunks = chunker.chunk_document("repo1", "hier.md", &content);
        let deep = chunks.iter().find(|c| c.heading_hierarchy.len() == 3);
        assert!(deep.is_some(), "expected a chunk with 3-level heading hierarchy");
        let h = &deep.unwrap().heading_hierarchy;
        assert_eq!(h[0], "Top");
        assert_eq!(h[1], "Sub");
        assert_eq!(h[2], "Deep");
    }
}
