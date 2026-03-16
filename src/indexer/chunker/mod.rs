mod hash;
mod parser;

use hash::sha256_hex;
use parser::{estimate_tokens, merge_small_sections, parse_sections, sliding_window};

#[derive(Debug, Clone)]
pub struct Chunk {
    pub repo_id: String,
    pub file_path: String,
    pub heading_hierarchy: Vec<String>,
    pub chunk_index: usize,
    pub chunk_hash: String,
    pub text: String,
    pub is_tombstone: bool,
}

pub struct MarkdownChunker {
    pub max_tokens: usize,
    pub min_tokens: usize,
    pub overlap_tokens: usize,
}

impl MarkdownChunker {
    pub fn new_from_env() -> Self {
        let max_tokens = std::env::var("PKS_CHUNK_MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(400);
        let min_tokens = std::env::var("PKS_CHUNK_MIN_TOKENS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);
        let overlap_tokens = std::env::var("PKS_CHUNK_OVERLAP_TOKENS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(80);
        Self { max_tokens, min_tokens, overlap_tokens }
    }

    pub fn chunk_document(&self, repo_id: &str, file_path: &str, content: &str) -> Vec<Chunk> {
        let sections = parse_sections(content);
        let sections = merge_small_sections(sections, self.min_tokens);

        let mut chunks = Vec::new();
        let mut index = 0;

        for section in sections {
            let tokens = estimate_tokens(&section.text);

            if tokens <= self.max_tokens {
                let hash = sha256_hex(&section.text);
                chunks.push(Chunk {
                    repo_id: repo_id.to_string(),
                    file_path: file_path.to_string(),
                    heading_hierarchy: section.heading_hierarchy,
                    chunk_index: index,
                    chunk_hash: hash,
                    text: section.text,
                    is_tombstone: false,
                });
                index += 1;
                continue;
            }

            let windows = sliding_window(&section.text, self.max_tokens, self.overlap_tokens);
            for window_text in windows {
                let hash = sha256_hex(&window_text);
                chunks.push(Chunk {
                    repo_id: repo_id.to_string(),
                    file_path: file_path.to_string(),
                    heading_hierarchy: section.heading_hierarchy.clone(),
                    chunk_index: index,
                    chunk_hash: hash,
                    text: window_text,
                    is_tombstone: false,
                });
                index += 1;
            }
        }

        chunks
    }

    pub fn tombstone(repo_id: &str, file_path: &str) -> Chunk {
        Chunk {
            repo_id: repo_id.to_string(),
            file_path: file_path.to_string(),
            heading_hierarchy: Vec::new(),
            chunk_index: 0,
            chunk_hash: sha256_hex(file_path),
            text: String::new(),
            is_tombstone: true,
        }
    }
}
