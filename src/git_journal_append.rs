use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::git_journal_date::{current_date_utc, unix_timestamp_to_hhmm};

#[derive(Debug)]
pub enum JournalAppendError {
    Git(git2::Error),
    Io(std::io::Error),
    MissingCommitData,
}

impl fmt::Display for JournalAppendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalAppendError::Git(e) => write!(f, "git error: {}", e),
            JournalAppendError::Io(e) => write!(f, "io error: {}", e),
            JournalAppendError::MissingCommitData => write!(f, "commit missing required data"),
        }
    }
}

impl From<git2::Error> for JournalAppendError {
    fn from(e: git2::Error) -> Self {
        JournalAppendError::Git(e)
    }
}

impl From<std::io::Error> for JournalAppendError {
    fn from(e: std::io::Error) -> Self {
        JournalAppendError::Io(e)
    }
}

pub struct JournalConfig {
    pub vault_root: String,
    pub enabled: bool,
    pub allow_prefixes: Vec<String>,
    pub min_words: usize,
    pub ignore_authors: Vec<String>,
}

impl JournalConfig {
    pub fn from_env() -> Self {
        let vault_root = std::env::var("PKS_VAULT_ROOT")
            .unwrap_or_else(|_| "prometheus".to_string());
        let enabled = std::env::var("PKS_GIT_LOG_ENABLED")
            .map(|v| v.to_lowercase() != "false")
            .unwrap_or(true);
        let allow_prefixes = std::env::var("PKS_GIT_ALLOW_PREFIXES")
            .unwrap_or_else(|_| "feat,fix,docs,perf,refactor,arch,test".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let min_words = std::env::var("PKS_GIT_MIN_WORDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let ignore_authors = std::env::var("PKS_GIT_IGNORE_AUTHORS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        JournalConfig { vault_root, enabled, allow_prefixes, min_words, ignore_authors }
    }
}

pub(crate) struct CommitMeta {
    pub sha7: String,
    pub author: String,
    pub time_hhmm: String,
    pub subject: String,
}

pub(crate) fn is_pks_knowledge_branch(branch: &str) -> bool {
    branch == "pks-knowledge"
}

pub(crate) fn passes_conventional_prefix(subject: &str, allow_prefixes: &[String]) -> bool {
    let colon_pos = match subject.find(':') {
        Some(pos) => pos,
        None => return false,
    };
    let raw_prefix = &subject[..colon_pos];
    let prefix = raw_prefix.split('(').next().unwrap_or("").trim();
    allow_prefixes.iter().any(|p| p == prefix)
}

pub(crate) fn passes_min_words(subject: &str, min_words: usize) -> bool {
    subject.split_whitespace().count() >= min_words
}

pub(crate) fn is_ignored_author(author: &str, ignore_authors: &[String]) -> bool {
    ignore_authors.iter().any(|a| a == author)
}

pub(crate) fn read_commit_metadata(repo_root: &Path, sha: &str) -> Result<CommitMeta, JournalAppendError> {
    let repo = git2::Repository::open(repo_root)?;
    let oid = git2::Oid::from_str(sha)?;
    let commit = repo.find_commit(oid)?;
    let sha7 = sha.get(..7).unwrap_or(sha).to_string();
    let author = commit.author().name()
        .ok_or(JournalAppendError::MissingCommitData)?
        .to_string();
    let time_hhmm = unix_timestamp_to_hhmm(commit.time().seconds());
    let subject = commit.summary()
        .ok_or(JournalAppendError::MissingCommitData)?
        .to_string();
    Ok(CommitMeta { sha7, author, time_hhmm, subject })
}

pub(crate) fn format_log_line(meta: &CommitMeta) -> String {
    format!("- **{}** - `{}` - {}: {}\n", meta.time_hhmm, meta.sha7, meta.author, meta.subject)
}

#[allow(dead_code)] // used in unit tests
pub(crate) fn daily_log_path(repo_root: &Path, vault_root: &str, date: &str) -> PathBuf {
    repo_root.join(vault_root).join("90-ai-memory").join(format!("{}_log.md", date))
}

#[allow(dead_code)] // used in unit tests
pub(crate) fn append_line_to_file(path: &PathBuf, line: &str) -> Result<(), JournalAppendError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

/// Returns the filename used for the journal entry in the pks-knowledge branch.
pub(crate) fn branch_log_filename(vault_root: &str, date: &str) -> String {
    format!("{vault_root}_journal_{date}.md")
}

/// Reads the current content of a flat file from the pks-knowledge branch, if it exists.
fn read_log_from_branch(repo_root: &Path, filename: &str) -> Option<String> {
    use crate::git::bare_commit::PKS_BRANCH;
    let repo = git2::Repository::open(repo_root).ok()?;
    let branch = repo.find_branch(PKS_BRANCH, git2::BranchType::Local).ok()?;
    let commit = branch.get().peel_to_commit().ok()?;
    let tree = commit.tree().ok()?;
    let entry = tree.get_name(filename)?;
    let blob = repo.find_blob(entry.id()).ok()?;
    std::str::from_utf8(blob.content()).ok().map(|s| s.to_owned())
}

/// Appends `line` to the daily journal file in the pks-knowledge branch via BareCommit.
pub(crate) fn append_line_to_branch(
    repo_root: &Path,
    vault_root: &str,
    line: &str,
    date: &str,
) -> Result<(), JournalAppendError> {
    use crate::git::BareCommit;
    let filename = branch_log_filename(vault_root, date);
    let bc = BareCommit::new(repo_root);
    bc.ensure_branch()?;
    let existing = read_log_from_branch(repo_root, &filename).unwrap_or_default();
    let new_content = format!("{existing}{line}");
    bc.write_file(&filename, new_content.as_bytes(), &format!("pks(journal): append commit to {date}"))?;
    Ok(())
}

pub fn append_commit_to_daily_log(
    repo_root: &Path,
    sha: &str,
    branch: &str,
    config: &JournalConfig,
) -> Result<(), JournalAppendError> {
    if !config.enabled {
        return Ok(());
    }
    if is_pks_knowledge_branch(branch) {
        return Ok(());
    }
    let meta = read_commit_metadata(repo_root, sha)?;
    if is_ignored_author(&meta.author, &config.ignore_authors) {
        return Ok(());
    }
    if !passes_conventional_prefix(&meta.subject, &config.allow_prefixes) {
        return Ok(());
    }
    if !passes_min_words(&meta.subject, config.min_words) {
        return Ok(());
    }
    let line = format_log_line(&meta);
    let date = current_date_utc();
    append_line_to_branch(repo_root, &config.vault_root, &line, &date)
}

#[cfg(test)]
#[path = "git_journal_append_tests.rs"]
mod tests;
