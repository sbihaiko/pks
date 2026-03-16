use super::*;

#[test]
fn pks_knowledge_branch_is_skipped() {
    assert!(is_pks_knowledge_branch("pks-knowledge"));
    assert!(!is_pks_knowledge_branch("main"));
    assert!(!is_pks_knowledge_branch("feature/foo"));
}

#[test]
fn conventional_prefix_accepted_with_scope() {
    let prefixes = vec!["feat".to_string(), "fix".to_string()];
    assert!(passes_conventional_prefix("feat(auth): add login", &prefixes));
    assert!(passes_conventional_prefix("fix: correct typo in readme", &prefixes));
}

#[test]
fn non_conventional_prefix_rejected() {
    let prefixes = vec!["feat".to_string(), "fix".to_string()];
    assert!(!passes_conventional_prefix("wip", &prefixes));
    assert!(!passes_conventional_prefix("update readme", &prefixes));
    assert!(!passes_conventional_prefix("docs: update readme", &prefixes));
}

#[test]
fn min_words_filter_works() {
    assert!(passes_min_words("feat: add user authentication flow", 5));
    assert!(!passes_min_words("fix: typo", 5));
    assert!(passes_min_words("a b c d e", 5));
    assert!(!passes_min_words("a b c d", 5));
}

#[test]
fn ignored_author_is_skipped() {
    let ignored = vec!["github-actions[bot]".to_string()];
    assert!(is_ignored_author("github-actions[bot]", &ignored));
    assert!(!is_ignored_author("bihaiko", &ignored));
    assert!(!is_ignored_author("other", &ignored));
}

#[test]
fn log_line_format_is_correct() {
    let meta = CommitMeta {
        sha7: "abc1234".to_string(),
        author: "bihaiko".to_string(),
        time_hhmm: "14:30".to_string(),
        subject: "feat: add login endpoint".to_string(),
    };
    let line = format_log_line(&meta);
    assert_eq!(line, "- **14:30** - `abc1234` - bihaiko: feat: add login endpoint\n");
}

#[test]
fn daily_log_path_uses_vault_root() {
    let root = std::path::PathBuf::from("/repo");
    let path = daily_log_path(&root, "prometheus", "2026-03-10");
    assert_eq!(path, std::path::PathBuf::from("/repo/prometheus/90-ai-memory/2026-03-10_log.md"));
}

#[test]
fn append_line_to_file_creates_parent_dirs() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("90-ai-memory/2026-03-10_log.md");
    append_line_to_file(&path, "- **10:00** - `abc1234` - author: feat: test\n").unwrap();
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("abc1234"));
}

#[test]
fn journal_config_defaults_when_env_absent() {
    std::env::remove_var("PKS_VAULT_ROOT");
    std::env::remove_var("PKS_GIT_LOG_ENABLED");
    std::env::remove_var("PKS_GIT_ALLOW_PREFIXES");
    std::env::remove_var("PKS_GIT_MIN_WORDS");
    std::env::remove_var("PKS_GIT_IGNORE_AUTHORS");
    let config = JournalConfig::from_env();
    assert_eq!(config.vault_root, "prometheus");
    assert!(config.enabled);
    assert_eq!(config.min_words, 5);
    assert!(config.ignore_authors.is_empty());
    assert!(config.allow_prefixes.contains(&"feat".to_string()));
}

#[test]
fn append_commit_skips_when_disabled() {
    let dir = tempfile::TempDir::new().unwrap();
    let config = JournalConfig {
        vault_root: "prometheus".to_string(),
        enabled: false,
        allow_prefixes: vec!["feat".to_string()],
        min_words: 1,
        ignore_authors: vec![],
    };
    let result = append_commit_to_daily_log(dir.path(), "abc1234", "main", &config);
    assert!(result.is_ok());
    assert!(!dir.path().join("prometheus/90-ai-memory").exists());
}

#[test]
fn append_commit_skips_pks_knowledge_branch() {
    let dir = tempfile::TempDir::new().unwrap();
    let config = JournalConfig {
        vault_root: "prometheus".to_string(),
        enabled: true,
        allow_prefixes: vec!["feat".to_string()],
        min_words: 1,
        ignore_authors: vec![],
    };
    let result = append_commit_to_daily_log(dir.path(), "abc1234", "pks-knowledge", &config);
    assert!(result.is_ok());
    assert!(!dir.path().join("prometheus/90-ai-memory").exists());
}
