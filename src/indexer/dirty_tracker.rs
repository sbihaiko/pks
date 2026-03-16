use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::indexer::chunker::Chunk;

pub fn compute_paragraph_hashes(text: &str) -> Vec<String> {
    text.split("\n\n")
        .map(|paragraph| {
            let mut hasher = Sha256::new();
            hasher.update(paragraph.as_bytes());
            format!("{:x}", hasher.finalize())
        })
        .collect()
}

pub fn mark_dirty_chunks(
    new_chunks: &[Chunk],
    cached_paragraph_hashes: &HashMap<String, Vec<String>>,
) -> Vec<bool> {
    new_chunks
        .iter()
        .map(|chunk| {
            let new_hashes = compute_paragraph_hashes(&chunk.text);
            let positional_key = format!("{}::{}", chunk.file_path, chunk.chunk_index);
            let cached = cached_paragraph_hashes.get(&positional_key);
            let Some(previous_hashes) = cached else {
                return true;
            };
            new_hashes != *previous_hashes
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraph_hash_changes_when_content_changes() {
        let original = "First paragraph content here.";
        let modified = "First paragraph content changed.";

        let hashes_original = compute_paragraph_hashes(original);
        let hashes_modified = compute_paragraph_hashes(modified);

        assert_ne!(hashes_original, hashes_modified);
    }

    #[test]
    fn same_paragraph_produces_same_hash() {
        let text = "Stable paragraph content.";

        let first = compute_paragraph_hashes(text);
        let second = compute_paragraph_hashes(text);

        assert_eq!(first, second);
    }

    #[test]
    fn multiple_paragraphs_split_correctly() {
        let text = "Paragraph one.\n\nParagraph two.\n\nParagraph three.";

        let hashes = compute_paragraph_hashes(text);

        assert_eq!(hashes.len(), 3);
    }
}
