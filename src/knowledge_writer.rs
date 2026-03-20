//! Shared helpers for writing ADRs and features to the pks-knowledge branch.

use crate::git::BareCommit;
use crate::ipc::{IpcClient, PksCommand};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddDecisionParams {
    /// The decision text / note to record as an ADR.
    pub note: String,
    /// Optional context or rationale for the decision.
    pub context: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddFeatureParams {
    /// Title of the feature specification.
    pub title: String,
    /// Feature description / specification content in Markdown.
    pub content: String,
    /// Optional tracker ID (e.g. "PAY-4421").
    pub tracker_id: Option<String>,
}

pub fn try_ipc_refresh() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    if let Ok(rt) = rt {
        let _ = rt.block_on(IpcClient::send_command(&PksCommand::Refresh { dry_run: false }));
    }
}

pub fn hash_8(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    input.hash(&mut h);
    format!("{:08x}", h.finish() & 0xFFFF_FFFF)
}

/// Truncates a string to at most `max_bytes` at a valid UTF-8 char boundary.
pub fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

pub fn build_decision_content(note: &str, timestamp: &str, source: &str, context: Option<&str>) -> String {
    let ctx = context
        .filter(|c| !c.trim().is_empty())
        .map(|c| format!("\n## Context\n\n{c}\n"))
        .unwrap_or_default();
    format!("---\ndate: {timestamp}\nsource: {source}\n---\n# {note}\n{ctx}")
}

pub fn build_feature_content(title: &str, body: &str, timestamp: &str, tracker_id: Option<&str>) -> String {
    let tracker_line = tracker_id
        .filter(|t| !t.trim().is_empty())
        .map(|t| format!("tracker_id: {t}\n"))
        .unwrap_or_default();
    format!("---\ndate: {timestamp}\nsource: mcp\n{tracker_line}---\n# {title}\n\n{body}")
}

pub fn decision_file_path(date_str: &str, hash: &str) -> String {
    format!("decisions/{date_str}_{hash}.md")
}

pub fn commit_to_vault(cwd: &std::path::Path, file_path: &str, content: &[u8], message: &str) -> Result<(), String> {
    let bc = BareCommit::new(cwd);
    bc.ensure_branch().map_err(|e| format!("ensure_branch: {e}"))?;
    bc.write_file(file_path, content, message).map_err(|e| format!("write_file: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_8_deterministic() {
        assert_eq!(hash_8("test"), hash_8("test"));
    }

    #[test]
    fn hash_8_differs() {
        assert_ne!(hash_8("A"), hash_8("B"));
    }

    #[test]
    fn safe_truncate_ascii() {
        assert_eq!(safe_truncate("hello world", 5), "hello");
    }

    #[test]
    fn safe_truncate_multibyte() {
        let s = "decisão técnica";
        let t = safe_truncate(s, 8);
        assert!(t.len() <= 8);
        assert!(t.is_char_boundary(t.len()));
    }

    #[test]
    fn safe_truncate_no_op_when_short() {
        assert_eq!(safe_truncate("hi", 100), "hi");
    }

    #[test]
    fn decision_content_has_frontmatter() {
        let c = build_decision_content("Use Rust", "2026-03-20T10:00:00Z", "cli", None);
        assert!(c.starts_with("---\n"));
        assert!(c.contains("source: cli"));
        assert!(c.contains("# Use Rust"));
    }

    #[test]
    fn decision_content_with_context() {
        let c = build_decision_content("Use Rust", "2026-03-20T10:00:00Z", "mcp", Some("Performance"));
        assert!(c.contains("## Context"));
        assert!(c.contains("Performance"));
    }

    #[test]
    fn feature_content_with_tracker() {
        let c = build_feature_content("Auth", "Login system", "2026-03-20T10:00:00Z", Some("PAY-123"));
        assert!(c.contains("tracker_id: PAY-123"));
        assert!(c.contains("# Auth"));
    }
}
