use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn sha256_hex_of_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension().map_or(false, |ext| ext == "md")
}

fn collect_md_files(vault: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(vault) else { return found; };
    let paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    for path in paths.iter().filter(|p| p.is_dir()) {
        found.extend(collect_md_files(path));
    }
    found.extend(paths.into_iter().filter(|p| is_markdown_file(p)));
    found
}

fn repo_id_from_path(vault: &Path) -> String {
    vault.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string())
}

fn drift_entry(vault: &Path, file_path: &Path, snapshot: &crate::snapshot::SnapshotData) -> Option<String> {
    let content = std::fs::read(file_path).ok()?;
    let disk_hash = sha256_hex_of_bytes(&content);
    let rel = file_path.strip_prefix(vault).unwrap_or(file_path);
    let rel_str = rel.to_string_lossy();
    let indexed = snapshot.chunks.iter()
        .any(|c| c.file_path.ends_with(rel_str.as_ref()) && c.chunk_hash == disk_hash);
    (!indexed).then(|| format!("DRIFT  {rel_str}"))
}

fn check_file_drift(vault: &Path, snapshot: &crate::snapshot::SnapshotData) -> Vec<String> {
    collect_md_files(vault)
        .iter()
        .filter_map(|f| drift_entry(vault, f, snapshot))
        .collect()
}

fn is_tombstone_chunk(chunk: &crate::snapshot::ChunkRecord) -> bool {
    chunk.chunk_text.is_empty() && chunk.heading_hierarchy.is_empty()
}

fn check_tombstone_residuals(snapshot: &crate::snapshot::SnapshotData) -> Vec<String> {
    snapshot.chunks.iter()
        .filter(|c| is_tombstone_chunk(c))
        .map(|c| format!("TOMBSTONE  {}", c.file_path))
        .collect()
}

fn check_vector_clock(vault: &Path, snapshot: &crate::snapshot::SnapshotData) -> Vec<String> {
    let mut issues = Vec::new();
    let repo = match git2::Repository::open(vault) {
        Ok(r) => r,
        Err(_) => return issues,
    };
    let Ok(head) = repo.head() else { return issues; };
    let Ok(commit) = head.peel_to_commit() else { return issues; };
    let real_sha = commit.id().to_string();
    let recorded_sha = &snapshot.vector_clock_sha;
    if &real_sha != recorded_sha && !recorded_sha.is_empty() {
        issues.push(format!("CLOCK_DRIFT  HEAD={real_sha} recorded={recorded_sha}"));
    }
    issues
}

pub fn run_validate(vault: &Path) -> i32 {
    let repo_id = repo_id_from_path(vault);
    let mgr = crate::snapshot::SnapshotManager::new_from_env();
    let snapshot = match mgr.read_snapshot_for_repo(&repo_id) {
        Ok(s) => s,
        Err(_) => {
            println!("validate: no snapshot found for repo '{repo_id}' — index may be empty.");
            return 1;
        }
    };

    let mut all_issues: Vec<String> = Vec::new();
    all_issues.extend(check_file_drift(vault, &snapshot));
    all_issues.extend(check_tombstone_residuals(&snapshot));
    all_issues.extend(check_vector_clock(vault, &snapshot));

    if all_issues.is_empty() {
        println!("validate: OK — no issues found in '{repo_id}'.");
        return 0;
    }
    println!("validate: {} issue(s) found in '{repo_id}':", all_issues.len());
    for issue in &all_issues {
        println!("  {issue}");
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn collect_md_files_finds_only_md_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A").unwrap();
        std::fs::write(dir.path().join("b.txt"), "text").unwrap();
        let found = collect_md_files(dir.path());
        assert_eq!(found.len(), 1);
        assert!(found[0].extension().map_or(false, |e| e == "md"));
    }

    #[test]
    fn run_validate_returns_one_when_no_snapshot() {
        let dir = TempDir::new().unwrap();
        let result = run_validate(dir.path());
        assert_eq!(result, 1);
    }
}
