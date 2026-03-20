use std::path::Path;

use chrono::Utc;

use crate::git::BareCommit;
use crate::ipc::{IpcClient, PksCommand};

/// Sanitizes a filename: replaces non-alphanumeric characters with `_`
/// and limits to 50 characters.
pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    sanitized.chars().take(50).collect()
}

fn build_frontmatter(agent: &str, timestamp: &str) -> String {
    format!("---\nagent: {agent}\ndate: {timestamp}\nsource: batch\n---\n")
}

fn journal_file_path(date_str: &str, agent: &str, sanitized: &str) -> String {
    format!("journals/{date_str}_{agent}_{sanitized}.md")
}

fn try_ipc_refresh() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    if let Ok(rt) = rt {
        let _ = rt.block_on(IpcClient::send_command(&PksCommand::Refresh { dry_run: false }));
    }
}

fn read_and_format(agent: &str, file: &Path) -> Option<(String, String, String)> {
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(e) => { eprintln!("pks submit-journal: failed to read {:?}: {e}", file); return None; }
    };
    let now = Utc::now();
    let full_content = format!("{}{content}", build_frontmatter(agent, &now.to_rfc3339()));
    let date_str = now.format("%Y-%m-%d").to_string();
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "journal".to_string());
    let file_path = journal_file_path(&date_str, agent, &sanitize_filename(&stem));
    let message = format!("chore(journal): add {agent} journal {date_str}");
    Some((full_content, file_path, message))
}

fn commit_journal(cwd: &Path, file_path: &str, content: &[u8], message: &str) -> bool {
    let bc = BareCommit::new(cwd);
    if let Err(e) = bc.write_file(file_path, content, message) {
        eprintln!("pks submit-journal: git write failed: {e}");
        return false;
    }
    true
}

/// Implements `pks submit-journal --agent <name> --file <path>`.
/// Always returns 0; errors are logged via eprintln and never block the agent.
pub fn run_submit_journal(agent: &str, file: &Path) -> i32 {
    let Some((full_content, file_path, message)) = read_and_format(agent, file) else { return 0 };
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => { eprintln!("pks submit-journal: cannot determine cwd: {e}"); return 0; }
    };
    commit_journal(&cwd, &file_path, full_content.as_bytes(), &message);
    try_ipc_refresh();
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use tempfile::TempDir;

    fn init_repo() -> TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "pks-test").unwrap();
        cfg.set_str("user.email", "pks@test.local").unwrap();
        tmp
    }

    #[test]
    fn frontmatter_is_prepended_correctly() {
        let fm = build_frontmatter("testagent", "2026-03-20T00:00:00Z");
        assert!(fm.starts_with("---\n"));
        assert!(fm.contains("agent: testagent\n"));
        assert!(fm.contains("date: 2026-03-20T00:00:00Z\n"));
        assert!(fm.contains("source: batch\n"));
        assert!(fm.ends_with("---\n"));
    }

    #[test]
    fn sanitize_filename_replaces_special_chars() {
        assert_eq!(sanitize_filename("my-file.md"), "my_file_md");
        assert_eq!(sanitize_filename("hello world!"), "hello_world_");
        assert_eq!(sanitize_filename("abc"), "abc");
    }

    #[test]
    fn sanitize_filename_limits_to_50_chars() {
        let long = "a".repeat(80);
        let result = sanitize_filename(&long);
        assert!(result.len() <= 50);
    }

    #[test]
    fn sanitize_filename_preserves_underscores_at_boundaries() {
        assert_eq!(sanitize_filename("_hello_"), "_hello_");
        assert_eq!(sanitize_filename("__test__"), "__test__");
    }

    fn commit_and_read_journal(tmp: &TempDir, agent: &str, stem: &str, body: &str) -> String {
        let bc = BareCommit::new(tmp.path());
        bc.ensure_branch().unwrap();
        let now = Utc::now();
        let date_str = now.format("%Y-%m-%d").to_string();
        let fm = build_frontmatter(agent, &now.to_rfc3339());
        let file_path = journal_file_path(&date_str, agent, &sanitize_filename(stem));
        bc.write_file(&file_path, format!("{fm}{body}").as_bytes(),
            &format!("chore(journal): add {agent} journal {date_str}")).unwrap();
        let repo = Repository::open(tmp.path()).unwrap();
        let branch = repo.find_branch("pks-knowledge", git2::BranchType::Local).unwrap();
        let tree = branch.get().peel_to_commit().unwrap().tree().unwrap();
        let jt = repo.find_tree(tree.get_name("journals").unwrap().id()).unwrap();
        let expected = format!("{date_str}_{agent}_{stem}.md");
        let blob = repo.find_blob(jt.get_name(&expected).unwrap().id()).unwrap();
        std::str::from_utf8(blob.content()).unwrap().to_string()
    }

    #[test]
    fn full_flow_commits_to_pks_knowledge() {
        let tmp = init_repo();
        let committed = commit_and_read_journal(&tmp, "antigravity", "res", "## Notes\nSome content here.\n");
        assert!(committed.contains("agent: antigravity"));
        assert!(committed.contains("source: batch"));
        assert!(committed.contains("## Notes"));
    }
}
