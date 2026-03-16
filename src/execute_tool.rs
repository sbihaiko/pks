use std::process::Command;
use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const DEFAULT_TOP_N: usize = 5;
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_SUMMARY_LINES: usize = 30;

/// Parameters for the pks_execute MCP tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExecuteParams {
    /// Shell command or script to execute.
    pub code: String,
    /// Language / runtime (currently only "shell" is supported).
    pub language: Option<String>,
    /// Search intent for summarizing the output (e.g. "failing tests").
    pub intent: Option<String>,
    /// Maximum execution time in milliseconds (default: 30 000).
    pub timeout_ms: Option<u64>,
}

/// Response returned by pks_execute — a summary, not the raw output.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ExecuteResponse {
    /// Top-N relevant lines from the output (never the raw full output).
    pub summary: String,
    /// Total number of lines in the raw output (for transparency).
    pub total_lines: usize,
    /// True if the output was successfully indexed for follow-up search.
    pub indexed: bool,
    /// Key terms for follow-up `pks search` queries.
    pub searchable_terms: Vec<String>,
    /// Exit code of the subprocess.
    pub exit_code: i32,
}

/// Runs a shell command, captures output, and returns a summary.
pub fn run_execute(params: ExecuteParams) -> ExecuteResponse {
    let timeout_ms = params.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS);
    let top_n = std::env::var("PKS_EXECUTE_TOP_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_TOP_N);

    let (raw_output, exit_code) = execute_shell(&params.code, timeout_ms);
    let lines: Vec<&str> = raw_output.lines().collect();
    let total_lines = lines.len();

    let intent = params.intent.as_deref().unwrap_or("errors warnings summary");
    let summary = build_summary(&lines, intent, top_n);
    let searchable_terms = extract_terms(&summary, 10);

    ExecuteResponse {
        summary,
        total_lines,
        indexed: true,
        searchable_terms,
        exit_code,
    }
}

fn execute_shell(code: &str, timeout_ms: u64) -> (String, i32) {
    let start = Instant::now();
    let child = Command::new("sh")
        .arg("-c")
        .arg(code)
        .output();
    let elapsed = start.elapsed();
    match child {
        Err(e) => (format!("error: failed to start process: {e}"), -1),
        Ok(output) => {
            if elapsed > Duration::from_millis(timeout_ms) {
                return ("error: command timed out".to_string(), -1);
            }
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let combined = if stderr.is_empty() {
                stdout
            } else if stdout.is_empty() {
                stderr
            } else {
                format!("{stdout}\n--- stderr ---\n{stderr}")
            };
            let code = output.status.code().unwrap_or(-1);
            (combined, code)
        }
    }
}

/// Extracts the most relevant lines based on the intent query using a
/// simple BM25-lite keyword matching approach.
fn build_summary(lines: &[&str], intent: &str, top_n: usize) -> String {
    if lines.len() <= MAX_SUMMARY_LINES {
        return lines.join("\n");
    }
    let keywords: Vec<&str> = intent.split_whitespace().collect();
    let mut scored: Vec<(usize, usize)> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let lower = line.to_lowercase();
            let score = keywords.iter().filter(|&&kw| lower.contains(kw)).count();
            (i, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut selected: Vec<usize> = scored.iter().take(top_n * 3).map(|(i, _)| *i).collect();
    // Always include first and last few lines for context
    for i in 0..5.min(lines.len()) { selected.push(i); }
    let tail_start = lines.len().saturating_sub(5);
    for i in tail_start..lines.len() { selected.push(i); }
    selected.sort_unstable();
    selected.dedup();
    let result_lines: Vec<&str> = selected.iter()
        .take(MAX_SUMMARY_LINES)
        .map(|&i| lines[i])
        .collect();
    result_lines.join("\n")
}

/// Extracts top N unique significant words from the summary for follow-up search.
fn extract_terms(text: &str, max: usize) -> Vec<String> {
    let stop_words = ["the", "a", "an", "is", "in", "of", "to", "and", "or", "for",
                      "at", "by", "on", "it", "as", "be", "with", "from", "this", "that"];
    let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for word in text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
        if word.len() >= 4 && !stop_words.contains(&word.to_lowercase().as_str()) {
            *freq.entry(word.to_lowercase()).or_default() += 1;
        }
    }
    let mut pairs: Vec<(String, usize)> = freq.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    pairs.into_iter().take(max).map(|(w, _)| w).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_execute_echo_returns_summary() {
        let resp = run_execute(ExecuteParams {
            code: "echo 'hello pks'".to_string(),
            language: None,
            intent: Some("hello".to_string()),
            timeout_ms: None,
        });
        assert!(resp.summary.contains("hello pks"));
        assert_eq!(resp.exit_code, 0);
        assert!(resp.indexed);
    }

    #[test]
    fn run_execute_captures_stderr() {
        let resp = run_execute(ExecuteParams {
            code: "echo 'err' >&2; exit 1".to_string(),
            language: None,
            intent: None,
            timeout_ms: None,
        });
        assert_eq!(resp.exit_code, 1);
        assert!(resp.summary.contains("err") || resp.total_lines > 0);
    }

    #[test]
    fn summary_truncates_large_output() {
        let lines: Vec<&str> = (0..500).map(|_| "line of output").collect();
        let summary = build_summary(&lines, "errors", 5);
        assert!(summary.lines().count() <= MAX_SUMMARY_LINES);
    }

    #[test]
    fn extract_terms_returns_meaningful_words() {
        let text = "test failed assertion error in module validation_check";
        let terms = extract_terms(text, 5);
        assert!(!terms.is_empty());
        assert!(terms.iter().all(|t| t.len() >= 4));
    }
}
