use std::collections::HashMap;

use crate::indexer::chunker::{Chunk, MarkdownChunker};
use crate::indexer::dirty_tracker::{compute_paragraph_hashes, mark_dirty_chunks};

pub struct IndexingPipeline {
    pub chunker: MarkdownChunker,
    pub paragraph_hash_cache: HashMap<String, Vec<String>>,
}

impl IndexingPipeline {
    pub fn new_from_env() -> Self {
        Self {
            chunker: MarkdownChunker::new_from_env(),
            paragraph_hash_cache: HashMap::new(),
        }
    }

    pub fn process_file(&self, repo_id: &str, file_path: &str, content: &str) -> Vec<Chunk> {
        self.chunker.chunk_document(repo_id, file_path, content)
    }

    pub fn process_deletion(&self, repo_id: &str, file_path: &str) -> Vec<Chunk> {
        vec![MarkdownChunker::tombstone(repo_id, file_path)]
    }

    pub fn process_file_with_dirty_markers(
        &mut self,
        repo_id: &str,
        file_path: &str,
        content: &str,
    ) -> Vec<(Chunk, bool)> {
        let chunks = self.chunker.chunk_document(repo_id, file_path, content);
        let dirty_flags = mark_dirty_chunks(&chunks, &self.paragraph_hash_cache);

        let tagged: Vec<(Chunk, bool)> = chunks
            .into_iter()
            .zip(dirty_flags.into_iter())
            .collect();

        for (chunk, _) in &tagged {
            let hashes = compute_paragraph_hashes(&chunk.text);
            let positional_key = format!("{}::{}", chunk.file_path, chunk.chunk_index);
            self.paragraph_hash_cache.insert(positional_key, hashes);
        }

        tagged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section_heading(index: usize) -> String {
        format!("## Section {}", index)
    }

    fn section_body(index: usize) -> String {
        format!(
            "Section {} contains stable body content designed to exceed the minimum token threshold. \
            This paragraph provides enough words so that the chunker treats each heading section as \
            its own independent chunk without merging adjacent sections together. \
            Unique marker alpha-{} beta-{} gamma-{} ensures hash distinctness.",
            index, index, index, index
        )
    }

    fn build_headed_document(bodies: &[String]) -> String {
        bodies
            .iter()
            .enumerate()
            .map(|(i, body)| format!("{}\n\n{}", section_heading(i), body))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[test]
    fn new_file_all_chunks_are_dirty() {
        let mut pipeline = IndexingPipeline::new_from_env();
        let bodies: Vec<String> = (0..5).map(|i| section_body(i)).collect();
        let content = build_headed_document(&bodies);

        let tagged = pipeline.process_file_with_dirty_markers("repo", "file.md", &content);

        assert!(!tagged.is_empty());
        assert!(tagged.iter().all(|(_, dirty)| *dirty));
    }

    #[test]
    fn unchanged_file_has_no_dirty_chunks() {
        let mut pipeline = IndexingPipeline::new_from_env();
        let bodies: Vec<String> = (0..5).map(|i| section_body(i)).collect();
        let content = build_headed_document(&bodies);

        pipeline.process_file_with_dirty_markers("repo", "file.md", &content);
        let second = pipeline.process_file_with_dirty_markers("repo", "file.md", &content);

        assert!(second.iter().all(|(_, dirty)| !dirty));
    }

    #[test]
    fn editing_one_paragraph_marks_only_affected_chunks_as_dirty() {
        let mut pipeline = IndexingPipeline::new_from_env();
        let mut bodies: Vec<String> = (0..5).map(|i| section_body(i)).collect();
        let original = build_headed_document(&bodies);

        let first = pipeline.process_file_with_dirty_markers("repo", "file.md", &original);
        assert!(first.iter().all(|(_, dirty)| *dirty));

        bodies[2] = "EDITED body for section 2 completely rewritten with different unique words here.".to_string();
        let modified = build_headed_document(&bodies);

        let second = pipeline.process_file_with_dirty_markers("repo", "file.md", &modified);

        let dirty_count = second.iter().filter(|(_, dirty)| *dirty).count();
        let clean_count = second.iter().filter(|(_, dirty)| !dirty).count();

        assert!(dirty_count >= 1, "At least one chunk must be dirty after edit");
        assert!(clean_count >= 1, "At least one chunk must remain clean");
    }
}
