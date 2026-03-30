use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum CheckStatus {
    Ok,
    Warn(String),
    Error(String),
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Ok => write!(f, "OK"),
            CheckStatus::Warn(m) => write!(f, "WARN: {m}"),
            CheckStatus::Error(m) => write!(f, "ERROR: {m}"),
        }
    }
}

#[derive(Debug)]
pub struct DoctorCheck {
    pub name: &'static str,
    pub status: CheckStatus,
    pub repaired: bool,
}

pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn all_ok(&self) -> bool {
        self.checks.iter().all(|c| c.status == CheckStatus::Ok)
    }

    pub fn exit_code(&self) -> i32 {
        if !self.all_ok() { return 1; }
        0
    }

    pub fn print(&self) {
        for check in &self.checks {
            let repaired = if check.repaired { " [REPAIRED]" } else { "" };
            println!("[{}] {}{}", check.status, check.name, repaired);
        }
    }
}

pub fn run_doctor(repo_root: &Path) -> DoctorReport {
    DoctorReport {
        checks: vec![
            check_worktree(repo_root),
            check_git_exclude(repo_root),
            check_post_commit_hook(repo_root),
            check_pks_knowledge_branch(repo_root),
            check_pending_commits(repo_root),
        ],
    }
}

fn repair_check(name: &'static str, repaired: bool, err_msg: &str) -> DoctorCheck {
    if !repaired {
        return DoctorCheck { name, status: CheckStatus::Error(err_msg.to_string()), repaired: false };
    }
    DoctorCheck { name, status: CheckStatus::Ok, repaired: true }
}

fn check_worktree(repo_root: &Path) -> DoctorCheck {
    use crate::git_branch::{worktree_exists, create_pks_branch_and_worktree};
    if worktree_exists(repo_root) {
        return DoctorCheck { name: "worktree:prometheus/", status: CheckStatus::Ok, repaired: false };
    }
    repair_check("worktree:prometheus/", create_pks_branch_and_worktree(repo_root).is_ok(), "worktree missing and repair failed")
}

fn check_git_exclude(repo_root: &Path) -> DoctorCheck {
    use crate::vault_init::add_to_git_exclude;
    let content = std::fs::read_to_string(repo_root.join(".git/info/exclude")).unwrap_or_default();
    if content.contains("prometheus/") {
        return DoctorCheck { name: ".git/info/exclude:prometheus/", status: CheckStatus::Ok, repaired: false };
    }
    repair_check(".git/info/exclude:prometheus/", add_to_git_exclude(repo_root).is_ok(), "could not write .git/info/exclude")
}

fn check_post_commit_hook(repo_root: &Path) -> DoctorCheck {
    use crate::vault_init::install_post_commit_hook;
    let hook_path = repo_root.join(".git/hooks/post-commit");
    if hook_path.exists() {
        let content = std::fs::read_to_string(&hook_path).unwrap_or_default();
        if content.contains("pks hook-post-commit") {
            return DoctorCheck { name: "hook:post-commit", status: CheckStatus::Ok, repaired: false };
        }
    }
    repair_check("hook:post-commit", install_post_commit_hook(repo_root).is_ok(), "hook installation failed")
}

fn check_pks_knowledge_branch(repo_root: &Path) -> DoctorCheck {
    use crate::git_branch::{branch_exists, PKS_BRANCH};
    if branch_exists(repo_root) {
        return DoctorCheck { name: "branch:pks-knowledge", status: CheckStatus::Ok, repaired: false };
    }
    DoctorCheck {
        name: "branch:pks-knowledge",
        status: CheckStatus::Warn(format!("branch '{PKS_BRANCH}' not found — run 'pks init'")),
        repaired: false,
    }
}

fn check_pending_commits(repo_root: &Path) -> DoctorCheck {
    let stale = repo_root.join(".git/pks_pending_commits.processing.jsonl");
    if stale.exists() {
        return DoctorCheck {
            name: "commit-event-log:no-stale-processing-file",
            status: CheckStatus::Warn(
                "stale .git/pks_pending_commits.processing.jsonl found — possible crash during processing".to_string(),
            ),
            repaired: false,
        };
    }
    DoctorCheck { name: "commit-event-log:no-stale-processing-file", status: CheckStatus::Ok, repaired: false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_git_dir(dir: &TempDir) -> std::path::PathBuf {
        let repo = dir.path().to_path_buf();
        std::fs::create_dir_all(repo.join(".git/info")).unwrap();
        std::fs::create_dir_all(repo.join(".git/hooks")).unwrap();
        repo
    }

    #[test]
    fn doctor_reports_missing_exclude_and_repairs() {
        let dir = TempDir::new().unwrap();
        let repo = make_git_dir(&dir);
        std::fs::write(repo.join(".git/info/exclude"), "").unwrap();
        let report = run_doctor(&repo);
        let check = report.checks.iter().find(|c| c.name.contains("exclude")).unwrap();
        assert_eq!(check.status, CheckStatus::Ok);
        assert!(check.repaired);
        let content = std::fs::read_to_string(repo.join(".git/info/exclude")).unwrap();
        assert!(content.contains("prometheus/"));
    }

    #[test]
    fn doctor_detects_missing_hook_and_installs() {
        let dir = TempDir::new().unwrap();
        let repo = make_git_dir(&dir);
        std::fs::write(repo.join(".git/info/exclude"), "prometheus/\n").unwrap();
        let report = run_doctor(&repo);
        let check = report.checks.iter().find(|c| c.name.contains("hook")).unwrap();
        assert_eq!(check.status, CheckStatus::Ok);
        assert!(check.repaired);
        let content = std::fs::read_to_string(repo.join(".git/hooks/post-commit")).unwrap();
        assert!(content.contains("pks hook-post-commit"));
    }

    #[test]
    fn report_exit_code_is_zero_when_all_ok() {
        let report = DoctorReport {
            checks: vec![
                DoctorCheck { name: "a", status: CheckStatus::Ok, repaired: false },
                DoctorCheck { name: "b", status: CheckStatus::Ok, repaired: true },
            ],
        };
        assert_eq!(report.exit_code(), 0);
        assert!(report.all_ok());
    }

    #[test]
    fn report_exit_code_is_one_when_errors() {
        let report = DoctorReport {
            checks: vec![
                DoctorCheck { name: "a", status: CheckStatus::Ok, repaired: false },
                DoctorCheck { name: "b", status: CheckStatus::Error("bad".to_string()), repaired: false },
            ],
        };
        assert_eq!(report.exit_code(), 1);
        assert!(!report.all_ok());
    }
}
