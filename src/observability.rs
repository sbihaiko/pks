use std::env;
use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

const DEFAULT_MAX_SIZE_BYTES: u64 = 52_428_800;
const LOG_FILE_NAME: &str = "pks.log";
const LOG_DIR_SUFFIX: &str = ".pks/logs";

pub struct LogConfig {
    pub log_dir: PathBuf,
    pub max_size_bytes: u64,
}

fn pks_logs_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(LOG_DIR_SUFFIX)
}

fn parse_max_size_bytes() -> u64 {
    let raw = env::var("PKS_LOG_MAX_SIZE").unwrap_or_default();
    raw.parse::<u64>().unwrap_or(DEFAULT_MAX_SIZE_BYTES)
}

pub fn log_config_from_env() -> LogConfig {
    LogConfig {
        log_dir: pks_logs_dir(),
        max_size_bytes: parse_max_size_bytes(),
    }
}

fn build_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
}

pub fn init_logging(config: &LogConfig) -> WorkerGuard {
    std::fs::create_dir_all(&config.log_dir).ok();

    let file_appender = tracing_appender::rolling::never(&config.log_dir, LOG_FILE_NAME);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let json_layer = fmt::layer()
        .json()
        .with_span_events(FmtSpan::NONE)
        .with_writer(non_blocking);

    tracing_subscriber::registry()
        .with(build_env_filter())
        .with(json_layer)
        .init();

    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_max_size_bytes_returns_parsed_value_for_valid_input() {
        let result: u64 = "10485760".parse::<u64>().unwrap_or(DEFAULT_MAX_SIZE_BYTES);
        assert_eq!(result, 10_485_760);
    }

    #[test]
    fn parse_max_size_bytes_returns_default_for_empty_input() {
        let result: u64 = "".parse::<u64>().unwrap_or(DEFAULT_MAX_SIZE_BYTES);
        assert_eq!(result, DEFAULT_MAX_SIZE_BYTES);
    }

    #[test]
    fn parse_max_size_bytes_returns_default_for_invalid_input() {
        let result: u64 = "not_a_number".parse::<u64>().unwrap_or(DEFAULT_MAX_SIZE_BYTES);
        assert_eq!(result, DEFAULT_MAX_SIZE_BYTES);
    }

    #[test]
    fn pks_logs_dir_ends_with_pks_logs_suffix() {
        let dir = pks_logs_dir();
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.ends_with(".pks/logs"), "Expected path to end with .pks/logs, got: {dir_str}");
    }

    #[test]
    fn log_config_default_max_size_is_50mb() {
        assert_eq!(DEFAULT_MAX_SIZE_BYTES, 52_428_800);
    }
}
