mod notion_client;
mod routing;

use std::path::Path;
use tracing::info;

use crate::tracker::sanitizer;
use crate::storage_policy::{default_policy, should_store, ContentType};
use routing::{dest_dir_from_routing, file_rel_path_for, synced_at_now};
use notion_client::{fetch_page, fetch_blocks, page_title, page_status, blocks_to_markdown};

pub use routing::file_rel_path_for as make_file_path;

#[derive(Debug)]
pub struct ImportedPage {
    pub content: String,
    pub file_rel_path: String,
    pub tracker_id: String,
}

#[derive(Debug)]
pub enum ImportError {
    NoToken,
    NotionApi(String),
    PolicyRejected(String),
    Io(std::io::Error),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::NoToken => write!(f, "NOTION_TOKEN not set"),
            ImportError::NotionApi(s) => write!(f, "Notion API error: {s}"),
            ImportError::PolicyRejected(s) => write!(f, "storage policy rejected: {s}"),
            ImportError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl From<std::io::Error> for ImportError {
    fn from(e: std::io::Error) -> Self {
        ImportError::Io(e)
    }
}

fn notion_token() -> Result<String, ImportError> {
    std::env::var("NOTION_TOKEN").map_err(|_| ImportError::NoToken)
}

fn enforce_storage_policy(policy: &crate::storage_policy::StoragePolicy, content: &str) -> Result<(), ImportError> {
    if should_store(policy, ContentType::TrackerImport, content) {
        return Ok(());
    }
    Err(ImportError::PolicyRejected(format!(
        "content size {} exceeds policy limit {}",
        content.len(),
        policy.max_size_bytes
    )))
}

fn build_frontmatter(tracker_id: &str, status: &str, synced_at: &str, source_sha: &str) -> String {
    format!(
        "---\ntracker_id: {tracker_id}\ntracker: notion\nstatus: {status}\ntags: []\nsynced_at: {synced_at}\nsource_commit_sha: {source_sha}\n---\n\n"
    )
}

pub async fn import_tracker_page(
    tracker_id: &str,
    prometheus_root: &Path,
) -> Result<ImportedPage, ImportError> {
    let token = notion_token()?;
    let source_sha = crate::git_branch::get_head_sha(prometheus_root)
        .unwrap_or_else(|| "unknown".to_string());

    let page = fetch_page(tracker_id, &token).await.map_err(ImportError::NotionApi)?;
    let blocks = fetch_blocks(&page.id, &token).await.map_err(ImportError::NotionApi)?;

    let title = page_title(&page);
    let status = page_status(&page);
    let frontmatter = build_frontmatter(tracker_id, &status, &synced_at_now(), &source_sha);
    let body_raw = format!("# {title}\n\n{}", blocks_to_markdown(&blocks));
    let body = sanitizer::sanitize_with_limit(&body_raw, sanitizer::max_import_size_bytes());
    let content = format!("{frontmatter}{body}");

    let policy = default_policy();
    enforce_storage_policy(&policy, &content)?;

    let dest_dir = dest_dir_from_routing(prometheus_root);
    let file_rel = file_rel_path_for(tracker_id, &dest_dir);

    info!(tracker_id, file_rel, "tracker page imported");

    Ok(ImportedPage { content, file_rel_path: file_rel, tracker_id: tracker_id.to_string() })
}

pub async fn import_and_commit(
    tracker_id: &str,
    prometheus_root: &Path,
) -> Result<ImportedPage, ImportError> {
    let imported = import_tracker_page(tracker_id, prometheus_root).await?;
    let source_sha = crate::git_branch::get_head_sha(prometheus_root)
        .unwrap_or_else(|| "unknown".to_string());
    let message = format!("feat(pks-knowledge): tracker import {tracker_id}");

    crate::git_branch::commit_to_pks_knowledge(
        prometheus_root,
        &imported.file_rel_path,
        &imported.content,
        &source_sha,
        &message,
    )
    .map_err(|e| ImportError::NotionApi(e.to_string()))?;

    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_frontmatter_contains_required_fields() {
        let fm = build_frontmatter("PAY-4421", "in_progress", "2026-03-06T14:00:00Z", "abc123");
        assert!(fm.starts_with("---\n"), "must start with frontmatter delimiter");
        assert!(fm.contains("tracker_id: PAY-4421"));
        assert!(fm.contains("tracker: notion"));
        assert!(fm.contains("status: in_progress"));
        assert!(fm.contains("tags: []"));
        assert!(fm.contains("synced_at: 2026-03-06T14:00:00Z"));
        assert!(fm.contains("source_commit_sha: abc123"));
    }

    #[test]
    fn build_frontmatter_ends_with_double_newline() {
        let fm = build_frontmatter("X-1", "done", "2026-01-01T00:00:00Z", "sha");
        assert!(fm.ends_with("---\n\n"), "must end with --- and blank line");
    }

    #[test]
    fn notion_token_returns_no_token_when_unset() {
        std::env::remove_var("NOTION_TOKEN");
        let result = notion_token();
        assert!(matches!(result, Err(ImportError::NoToken)));
    }

    #[test]
    #[ignore]
    fn import_tracker_page_requires_notion_token() {
        std::env::remove_var("NOTION_TOKEN");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let result = rt.block_on(import_tracker_page("PAY-4421", tmp.path()));
        assert!(matches!(result, Err(ImportError::NoToken)));
    }
}
