//! Integration tests for M13 pks_execute tool.

use pks::execute_tool::{ExecuteParams, run_execute};

#[test]
fn execute_large_output_returns_bounded_summary() {
    // Generate 500 lines of output
    let resp = run_execute(ExecuteParams {
        code: "for i in $(seq 1 500); do echo \"line $i\"; done".to_string(),
        language: None,
        intent: Some("errors warnings".to_string()),
        timeout_ms: Some(10_000),
    });

    assert!(resp.total_lines >= 500, "must capture all 500 lines, got {}", resp.total_lines);
    assert!(resp.indexed, "output must be indexed");
    assert!(
        resp.summary.len() < 2000,
        "summary must be <2000 chars, got {} chars",
        resp.summary.len()
    );
    assert_eq!(resp.exit_code, 0);
}

#[test]
fn execute_with_intent_filters_relevant_lines() {
    let resp = run_execute(ExecuteParams {
        code: "echo 'PASSED: all tests ok'; echo 'FAILED: auth_test'; echo 'some other output'".to_string(),
        language: None,
        intent: Some("FAILED".to_string()),
        timeout_ms: None,
    });
    assert!(resp.summary.contains("FAILED"), "summary must contain FAILED line");
    assert_eq!(resp.exit_code, 0);
}

#[test]
fn execute_nonzero_exit_captured() {
    let resp = run_execute(ExecuteParams {
        code: "exit 42".to_string(),
        language: None,
        intent: None,
        timeout_ms: None,
    });
    assert_eq!(resp.exit_code, 42);
}

#[test]
fn execute_short_output_returned_verbatim() {
    let resp = run_execute(ExecuteParams {
        code: "echo 'short output'".to_string(),
        language: None,
        intent: None,
        timeout_ms: None,
    });
    assert!(resp.summary.contains("short output"));
    assert_eq!(resp.total_lines, 1);
}
