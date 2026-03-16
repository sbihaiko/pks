use crate::repo_watcher::RepoWatcher;
use std::collections::HashSet;

pub struct RefreshCommand;

impl RefreshCommand {
    pub fn run(dry_run: bool) -> i32 {
        let vaults_dir = RepoWatcher::vaults_dir_from_env();
        let (tx, _rx) = std::sync::mpsc::channel();
        let watcher = RepoWatcher::new(vaults_dir, tx);
        let found_repos = watcher.scan_existing_repos();
        let found_names: HashSet<String> = found_repos
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();

        if found_names.is_empty() {
            println!("pks refresh: no repositories found in vaults dir");
            return 0;
        }

        let mut sorted: Vec<&String> = found_names.iter().collect();
        sorted.sort();
        for name in sorted {
            println!("[=] {name}");
        }

        if dry_run {
            println!("pks refresh --dry-run: no changes applied");
        } else {
            println!("pks refresh: done ({} repos)", found_names.len());
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refresh_run_no_vaults_dir() {
        // Point PKS_VAULTS_DIR at a path that doesn't exist
        let nonexistent = "/tmp/pks-test-nonexistent-vaults-dir-xyz";
        std::env::set_var("PKS_VAULTS_DIR", nonexistent);
        let exit_code = RefreshCommand::run(false);
        std::env::remove_var("PKS_VAULTS_DIR");
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_refresh_dry_run_flag() {
        // Point at a non-existent dir so scan returns empty — no panic expected
        let nonexistent = "/tmp/pks-test-dry-run-dir-xyz";
        std::env::set_var("PKS_VAULTS_DIR", nonexistent);
        let exit_code = RefreshCommand::run(true);
        std::env::remove_var("PKS_VAULTS_DIR");
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_refresh_with_repos() {
        let tmp = tempfile::tempdir().unwrap();
        let vaults_dir = tmp.path().to_path_buf();
        let repo_a = vaults_dir.join("repo-a");
        std::fs::create_dir_all(repo_a.join(".git")).unwrap();

        std::env::set_var("PKS_VAULTS_DIR", vaults_dir.to_str().unwrap());
        let exit_code = RefreshCommand::run(false);
        std::env::remove_var("PKS_VAULTS_DIR");
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_refresh_dry_run_with_repos() {
        let tmp = tempfile::tempdir().unwrap();
        let vaults_dir = tmp.path().to_path_buf();
        let repo_b = vaults_dir.join("repo-b");
        std::fs::create_dir_all(repo_b.join(".git")).unwrap();

        std::env::set_var("PKS_VAULTS_DIR", vaults_dir.to_str().unwrap());
        let exit_code = RefreshCommand::run(true);
        std::env::remove_var("PKS_VAULTS_DIR");
        assert_eq!(exit_code, 0);
    }
}
