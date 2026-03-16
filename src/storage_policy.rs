/// Storage policy for the prometheus/ directory.
///
/// Decides what content is stored based on type, size, and tags.
/// Principle: "prometheus/ stores context and reasoning, not raw data."

const DEFAULT_MAX_SIZE: usize = 1_048_576; // 1 MB

#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    TrackerImport,
    AiSummary,
    Adr,
    ManualNote,
    Runbook,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct StoragePolicy {
    pub max_size_bytes: usize,
    pub allowed_types: Vec<ContentType>,
}

pub fn default_policy() -> StoragePolicy {
    StoragePolicy {
        max_size_bytes: max_content_size_bytes(),
        allowed_types: vec![
            ContentType::TrackerImport,
            ContentType::AiSummary,
            ContentType::Adr,
            ContentType::ManualNote,
            ContentType::Runbook,
        ],
    }
}

pub fn max_content_size_bytes() -> usize {
    std::env::var("PKS_IMPORT_MAX_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_SIZE)
}

pub fn content_type_from_tags(tags: &[&str]) -> ContentType {
    for tag in tags {
        let lower = tag.to_lowercase();
        if lower.contains("tracker") || lower.contains("import") {
            return ContentType::TrackerImport;
        }
        if lower.contains("ai-summary") || lower.contains("session-summary") {
            return ContentType::AiSummary;
        }
        if lower.contains("adr") || lower.contains("decision") {
            return ContentType::Adr;
        }
        if lower.contains("runbook") || lower.contains("playbook") {
            return ContentType::Runbook;
        }
        if lower.contains("note") || lower.contains("manual") {
            return ContentType::ManualNote;
        }
    }
    ContentType::Unknown
}

pub fn should_store(
    policy: &StoragePolicy,
    content_type: ContentType,
    content: &str,
) -> bool {
    if content.len() > policy.max_size_bytes {
        return false;
    }
    policy.allowed_types.contains(&content_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_stores_tracker_imports() {
        let policy = default_policy();
        let result = should_store(
            &policy,
            ContentType::TrackerImport,
            "tracker data",
        );
        assert!(result);
    }

    #[test]
    fn default_policy_stores_ai_summaries() {
        let policy = default_policy();
        let result = should_store(
            &policy,
            ContentType::AiSummary,
            "session summary content",
        );
        assert!(result);
    }

    #[test]
    fn oversized_content_is_rejected() {
        let policy = StoragePolicy {
            max_size_bytes: 10,
            allowed_types: vec![ContentType::ManualNote],
        };
        let big = "a".repeat(11);
        assert!(!should_store(&policy, ContentType::ManualNote, &big));
    }

    #[test]
    fn content_type_from_tags_detects_tracker() {
        let tags = vec!["tracker-import", "weekly"];
        assert_eq!(
            content_type_from_tags(&tags),
            ContentType::TrackerImport,
        );
    }

    #[test]
    fn content_type_from_tags_defaults_to_unknown() {
        let tags = vec!["random", "stuff"];
        assert_eq!(content_type_from_tags(&tags), ContentType::Unknown);
    }

    #[test]
    fn should_store_respects_size_limit() {
        let policy = StoragePolicy {
            max_size_bytes: 5,
            allowed_types: vec![ContentType::Adr],
        };
        assert!(should_store(&policy, ContentType::Adr, "hi"));
        assert!(!should_store(&policy, ContentType::Adr, "toolong"));
    }
}
