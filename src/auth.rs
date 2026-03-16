use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Deferred security items: per-token auth, TLS termination, CORS multi-tenant
/// policies, and API key rotation are not yet implemented. These are tracked as
/// M7+ backlog. Current surface is loopback-only (127.0.0.1) which limits
/// exposure without additional network controls.
pub struct SecurityTechDebt;

pub fn validate_embedding_debt_jsonl(path: &Path) -> Result<usize, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("cannot open embedding_debt.jsonl: {e}"))?;
    let reader = BufReader::new(file);
    let mut valid_count = 0usize;
    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result
            .map_err(|e| format!("read error at line {line_num}: {e}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        serde_json::from_str::<serde_json::Value>(trimmed)
            .map_err(|e| format!("invalid JSON at line {line_num}: {e}"))?;
        valid_count += 1;
    }
    Ok(valid_count)
}

pub fn validate_snapshot_bin_header(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("cannot read snapshot file: {e}"))?;
    let magic = crate::snapshot::MAGIC;
    if bytes.len() < magic.len() {
        return Err("snapshot file too small to contain magic header".to_string());
    }
    if &bytes[..magic.len()] != magic.as_ref() {
        return Err("snapshot magic header mismatch — file may be corrupt or invalid".to_string());
    }
    Ok(())
}
