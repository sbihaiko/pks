use chrono::Utc;

use crate::knowledge_writer::{
    build_decision_content, commit_to_vault, decision_file_path,
    hash_8, safe_truncate, try_ipc_refresh,
};

/// Implements `pks decision "<note>"`.
/// Writes an ADR to `decisions/` on the pks-knowledge branch.
pub fn run_decision(note: &str) -> i32 {
    if note.trim().is_empty() {
        eprintln!("pks decision: note cannot be empty. Usage: pks decision \"<note>\"");
        return 1;
    }
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => { eprintln!("pks decision: cannot determine cwd: {e}"); return 1; }
    };
    let now = Utc::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let hash = hash_8(note);
    let content = build_decision_content(note, &now.to_rfc3339(), "cli", None);
    let file_path = decision_file_path(&date_str, &hash);
    let message = format!("pks(decision): {}", safe_truncate(note, 60));

    if let Err(e) = commit_to_vault(&cwd, &file_path, content.as_bytes(), &message) {
        eprintln!("pks decision: {e}");
        return 1;
    }
    println!("✓ Decision recorded: {file_path}");
    try_ipc_refresh();
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::BareCommit;
    use crate::knowledge_writer::{build_decision_content, decision_file_path, hash_8};
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
    fn build_content_has_frontmatter() {
        let c = build_decision_content("Test", "2026-03-20T10:00:00Z", "cli", None);
        assert!(c.starts_with("---\n"));
        assert!(c.contains("source: cli"));
        assert!(c.contains("# Test"));
    }

    #[test]
    fn decision_file_path_format() {
        let p = decision_file_path("2026-03-20", "abcd1234");
        assert_eq!(p, "decisions/2026-03-20_abcd1234.md");
    }

    #[test]
    fn full_flow_writes_to_pks_knowledge() {
        let tmp = init_repo();
        let bc = BareCommit::new(tmp.path());
        bc.ensure_branch().unwrap();

        let now = Utc::now();
        let date_str = now.format("%Y-%m-%d").to_string();
        let hash = hash_8("Usar Rust 2024");
        let content = build_decision_content("Usar Rust 2024", &now.to_rfc3339(), "cli", None);
        let file_path = decision_file_path(&date_str, &hash);

        bc.write_file(&file_path, content.as_bytes(), "test decision").unwrap();

        let repo = Repository::open(tmp.path()).unwrap();
        let branch = repo.find_branch("pks-knowledge", git2::BranchType::Local).unwrap();
        let tree = branch.get().peel_to_commit().unwrap().tree().unwrap();
        let decisions_tree = repo.find_tree(tree.get_name("decisions").unwrap().id()).unwrap();
        let expected_name = format!("{date_str}_{hash}.md");
        let entry = decisions_tree.get_name(&expected_name);
        assert!(entry.is_some(), "decision file must exist in pks-knowledge");
        let blob = repo.find_blob(entry.unwrap().id()).unwrap();
        let text = std::str::from_utf8(blob.content()).unwrap();
        assert!(text.contains("source: cli"));
        assert!(text.contains("# Usar Rust 2024"));
    }
}
