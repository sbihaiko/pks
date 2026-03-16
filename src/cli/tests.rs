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

#[test]
fn parse_refresh_without_dry_run() {
    let cmd = parse_args(&args(&["pks", "refresh"]));
    let CliCommand::Refresh { dry_run } = cmd else { panic!("wrong variant"); };
    assert!(!dry_run);
}

#[test]
fn parse_refresh_with_dry_run_flag() {
    let cmd = parse_args(&args(&["pks", "refresh", "--dry-run"]));
    let CliCommand::Refresh { dry_run } = cmd else { panic!("wrong variant"); };
    assert!(dry_run);
}

#[test]
fn parse_daemon_not_in_parse_args() {
    // --daemon is a top-level flag handled in main(), not a CLI subcommand.
    // parse_args should treat it as Unknown.
    let cmd = parse_args(&args(&["pks", "--daemon"]));
    assert!(matches!(cmd, CliCommand::Unknown(_)));
}
