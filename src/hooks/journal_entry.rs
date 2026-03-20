use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Raw event from a single tool invocation.
#[derive(Debug, Clone, Serialize)]
pub struct ToolEvent {
    pub tool_name: String,
    pub input_summary: String,
    pub outcome: String,
    pub file_paths: Vec<String>,
    pub decision_note: Option<String>,
}

/// One persisted journal entry derived from a ToolEvent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub timestamp: DateTime<Utc>,
    pub tool_name: String,
    pub tool_input_summary: String,
    pub outcome: String,
    pub file_paths: Vec<String>,
    pub decision_note: Option<String>,
}

/// Shadow journal configuration loaded from environment variables.
pub struct JournalConfig {
    pub enabled: bool,
    pub min_words: usize,
    pub max_entries: usize,
    pub truncate_chars: usize,
}

impl Default for JournalConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("PKS_SHADOW_JOURNAL")
                .map(|v| v.to_lowercase() != "false")
                .unwrap_or(true),
            min_words: parse_env("PKS_JOURNAL_MIN_WORDS", 10),
            max_entries: parse_env("PKS_JOURNAL_MAX_ENTRIES", 500),
            truncate_chars: parse_env("PKS_JOURNAL_TRUNCATE", 200),
        }
    }
}

fn parse_env(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Truncates `s` to at most `max_chars` characters, appending "..." if cut.
pub fn truncate_summary(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{cut}...")
    }
}

/// Redacts common secret patterns (API keys, bearer tokens) from a string.
pub fn redact_secrets(s: String) -> String {
    use std::sync::OnceLock;
    static PATTERNS: OnceLock<Vec<(regex_lite::Regex, &'static str)>> = OnceLock::new();
    let patterns = PATTERNS.get_or_init(|| {
        [
            (r"sk-[A-Za-z0-9_-]{20,}", "[REDACTED_API_KEY]"),
            (r"Bearer [A-Za-z0-9._-]{10,}", "[REDACTED_BEARER]"),
            (r"password=[^\s&]{4,}", "[REDACTED_PASSWORD]"),
            (r"token=[^\s&]{4,}", "[REDACTED_TOKEN]"),
        ]
        .into_iter()
        .filter_map(|(pat, rep)| regex_lite::Regex::new(pat).ok().map(|r| (r, rep)))
        .collect()
    });
    let mut result = s;
    for (re, replacement) in patterns {
        result = re.replace_all(&result, *replacement).into_owned();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_summary("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string_appends_ellipsis() {
        let s = "a".repeat(210);
        let r = truncate_summary(&s, 200);
        assert!(r.len() <= 200);
        assert!(r.ends_with("..."));
    }

    #[test]
    fn redact_secrets_removes_api_keys() {
        let input = "key=sk-abc123def456ghi789jkl0 ok".to_string();
        let out = redact_secrets(input);
        assert!(!out.contains("sk-abc123"), "API key must be redacted");
        assert!(out.contains("[REDACTED_API_KEY]"));
    }

    #[test]
    fn redact_secrets_leaves_normal_text() {
        let input = "no secrets here".to_string();
        assert_eq!(redact_secrets(input), "no secrets here");
    }
}
