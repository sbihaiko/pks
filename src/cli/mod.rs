mod status;
mod validate;

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum CliCommand {
    Init { path: PathBuf },
    Doctor { path: PathBuf },
    HookPostCommit { path: PathBuf, sha: String, branch: String },
    Status { port: u16 },
    Validate { path: PathBuf },
    Unknown(Vec<String>),
}

pub fn parse_args(args: &[String]) -> CliCommand {
    match args.get(1).map(|s| s.as_str()) {
        Some("init") => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            CliCommand::Init { path }
        }
        Some("doctor") => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            CliCommand::Doctor { path }
        }
        Some("hook-post-commit") => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            let sha = args.get(3).cloned().unwrap_or_default();
            let branch = args.get(4).cloned().unwrap_or_else(|| "main".to_string());
            CliCommand::HookPostCommit { path, sha, branch }
        }
        Some("status") => {
            let port = args.get(2)
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(3030);
            CliCommand::Status { port }
        }
        Some("validate") => {
            let path = args.get(2).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            CliCommand::Validate { path }
        }
        _ => CliCommand::Unknown(args.to_vec()),
    }
}

pub async fn run_command(cmd: CliCommand) -> i32 {
    match cmd {
        CliCommand::Init { path } => run_init(&path),
        CliCommand::Doctor { path } => run_doctor(&path),
        CliCommand::HookPostCommit { path, sha, branch } => run_hook_post_commit(&path, &sha, &branch),
        CliCommand::Status { port } => status::run_status(port).await,
        CliCommand::Validate { path } => validate::run_validate(&path),
        CliCommand::Unknown(args) => {
            eprintln!("pks: unknown command. Args: {:?}", &args[1..]);
            eprintln!("Usage: pks <init|doctor|hook-post-commit|status|validate> [path]");
            1
        }
    }
}

fn run_init_post_steps(path: &Path) {
    use crate::vault_init::{add_to_git_exclude, install_post_commit_hook};
    if let Err(e) = add_to_git_exclude(path) {
        eprintln!("pks init warning: could not update .git/info/exclude: {e}");
    }
    if let Err(e) = install_post_commit_hook(path) {
        eprintln!("pks init warning: could not install post-commit hook: {e}");
    }
    if let Err(e) = crate::git_branch::create_pks_branch_and_worktree(path) {
        eprintln!("pks init warning: could not set up pks-knowledge branch: {e}");
    }
}

fn run_init(path: &Path) -> i32 {
    use crate::vault_init::init_vault;
    let result = match init_vault(path) {
        Ok(r) => r,
        Err(e) => { eprintln!("pks init error: {e}"); return 1; }
    };
    if result.was_idempotent {
        println!("pks init: vault already initialized (idempotent).");
        return 0;
    }
    println!("pks init: created {} directories.", result.dirs_created.len());
    run_init_post_steps(path);
    println!("pks init: done.");
    0
}

fn run_doctor(path: &Path) -> i32 {
    let report = crate::doctor::run_doctor(path);
    report.print();
    report.exit_code()
}

fn run_hook_post_commit(path: &Path, sha: &str, branch: &str) -> i32 {
    tracing::info!(
        repo_path = %path.display(),
        sha = %sha,
        branch = %branch,
        "post-commit hook triggered"
    );
    let trigger_file = path.join(".git/pks_hook_trigger");
    let payload = format!("{}:{}\n", sha, branch);
    let _ = std::fs::write(trigger_file, payload);
    let config = crate::git_journal_append::JournalConfig::from_env();
    if let Err(e) = crate::git_journal_append::append_commit_to_daily_log(path, sha, branch, &config) {
        tracing::warn!(error = %e, "git journal append failed (non-blocking)");
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|&a| a.to_string()).collect()
    }

    #[test]
    fn parse_init_command() {
        let cmd = parse_args(&args(&["pks", "init", "/tmp/repo"]));
        assert!(matches!(cmd, CliCommand::Init { .. }));
    }

    #[test]
    fn parse_doctor_command() {
        let cmd = parse_args(&args(&["pks", "doctor", "/tmp/repo"]));
        assert!(matches!(cmd, CliCommand::Doctor { .. }));
    }

    #[test]
    fn parse_hook_post_commit() {
        let cmd = parse_args(&args(&["pks", "hook-post-commit", "/tmp/repo", "abc123", "main"]));
        let CliCommand::HookPostCommit { sha, branch, .. } = cmd else {
            panic!("wrong command parsed");
        };
        assert_eq!(sha, "abc123");
        assert_eq!(branch, "main");
    }

    #[test]
    fn parse_unknown_returns_unknown_variant() {
        let cmd = parse_args(&args(&["pks", "foobar"]));
        assert!(matches!(cmd, CliCommand::Unknown(_)));
    }

    #[tokio::test]
    async fn run_unknown_returns_exit_code_1() {
        let cmd = CliCommand::Unknown(args(&["pks", "foobar"]));
        assert_eq!(run_command(cmd).await, 1);
    }

    #[test]
    fn parse_status_uses_default_port_3030() {
        let cmd = parse_args(&args(&["pks", "status"]));
        let CliCommand::Status { port } = cmd else { panic!("wrong variant"); };
        assert_eq!(port, 3030);
    }

    #[test]
    fn parse_status_accepts_custom_port() {
        let cmd = parse_args(&args(&["pks", "status", "8080"]));
        let CliCommand::Status { port } = cmd else { panic!("wrong variant"); };
        assert_eq!(port, 8080);
    }

    #[test]
    fn parse_validate_uses_current_dir_as_default() {
        let cmd = parse_args(&args(&["pks", "validate"]));
        let CliCommand::Validate { path } = cmd else { panic!("wrong variant"); };
        assert_eq!(path, std::path::PathBuf::from("."));
    }

    #[test]
    fn parse_validate_accepts_explicit_path() {
        let cmd = parse_args(&args(&["pks", "validate", "/tmp/vault"]));
        let CliCommand::Validate { path } = cmd else { panic!("wrong variant"); };
        assert_eq!(path, std::path::PathBuf::from("/tmp/vault"));
    }
}
