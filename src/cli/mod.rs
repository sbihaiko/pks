pub mod flush_session;
pub mod init;
pub mod record_event;
mod status;
pub mod submit_journal;
mod validate;

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum CliCommand {
    Init { path: PathBuf, force: bool },
    Doctor { path: PathBuf },
    HookPostCommit { path: PathBuf, sha: String, branch: String },
    Status { port: u16 },
    Validate { path: PathBuf },
    Refresh { dry_run: bool },
    FlushSession { session_id: String, cwd: PathBuf },
    RecordEvent,
    SubmitJournal { agent: String, file: PathBuf },
    Unknown(Vec<String>),
}

pub fn parse_args(args: &[String]) -> CliCommand {
    match args.get(1).map(|s| s.as_str()) {
        Some("init") => {
            let force = args.iter().any(|a| a == "--force");
            let path = args.iter().skip(2).find(|a| !a.starts_with("--"))
                .map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            CliCommand::Init { path, force }
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
        Some("refresh") => {
            let dry_run = args.iter().any(|a| a == "--dry-run");
            CliCommand::Refresh { dry_run }
        }
        Some("flush-session") => {
            let session_id = args.get(2).cloned().unwrap_or_default();
            let cwd = args.get(3).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            CliCommand::FlushSession { session_id, cwd }
        }
        Some("record-event") => CliCommand::RecordEvent,
        Some("submit-journal") => {
            let agent = parse_flag_value(args, "--agent").unwrap_or_default();
            let file = parse_flag_value(args, "--file")
                .map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
            CliCommand::SubmitJournal { agent, file }
        }
        _ => CliCommand::Unknown(args.to_vec()),
    }
}

pub async fn run_command(cmd: CliCommand) -> i32 {
    match cmd {
        CliCommand::Init { path, force } => run_init(&path, force),
        CliCommand::Doctor { path } => run_doctor(&path),
        CliCommand::HookPostCommit { path, sha, branch } => run_hook_post_commit(&path, &sha, &branch),
        CliCommand::Status { port } => status::run_status(port).await,
        CliCommand::Validate { path } => validate::run_validate(&path),
        CliCommand::Refresh { dry_run } => run_refresh(dry_run),
        CliCommand::FlushSession { session_id, cwd } => {
            flush_session::run_flush_session(&session_id, &cwd)
        }
        CliCommand::RecordEvent => record_event::run_record_event(),
        CliCommand::SubmitJournal { agent, file } => {
            submit_journal::run_submit_journal(&agent, &file)
        }
        CliCommand::Unknown(args) => {
            eprintln!("pks: unknown command. Args: {:?}", &args[1..]);
            eprintln!("Usage: pks <init|doctor|hook-post-commit|status|validate|refresh|record-event|flush-session|submit-journal> [args]");
            1
        }
    }
}

fn run_init(path: &Path, force: bool) -> i32 {
    use crate::cli::init::{InitCommand, InitError};
    let cmd = InitCommand::new(path.to_path_buf(), force);
    match cmd.run() {
        Ok(()) => 0,
        Err(InitError::AlreadyInitialized) => {
            eprintln!("⚠ PKS já inicializado em {}\n  Use --force para sobrescrever a configuração existente.", path.display());
            1
        }
        Err(e) => { eprintln!("✗ Erro: {e}"); 1 }
    }
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

fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

fn run_refresh(dry_run: bool) -> i32 {
    crate::commands::refresh::RefreshCommand::run(dry_run)
}

#[cfg(test)]
mod tests;
