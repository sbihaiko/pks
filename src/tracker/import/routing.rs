use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_DEST_DIR: &str = "02-features";
const ROUTING_CONFIG_FILE: &str = ".pks-routing.yaml";

fn extract_dest_dir(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let stripped = trimmed.strip_prefix("tracker_import:")?;
    let val = stripped.trim();
    (!val.is_empty()).then(|| val.to_string())
}

pub fn dest_dir_from_routing(prometheus_root: &Path) -> String {
    let routing_path = prometheus_root.join(ROUTING_CONFIG_FILE);
    let contents = std::fs::read_to_string(&routing_path).unwrap_or_default();
    contents
        .lines()
        .find_map(extract_dest_dir)
        .unwrap_or_else(|| DEFAULT_DEST_DIR.to_string())
}

pub fn file_rel_path_for(tracker_id: &str, dest_dir: &str) -> String {
    let safe_id = tracker_id.replace('/', "-").replace(' ', "_");
    format!("{dest_dir}/{safe_id}.md")
}

pub fn synced_at_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, mo, d, h, mi, s) = secs_to_parts(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn secs_to_parts(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let mi = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let (y, mo, d) = days_to_ymd(days);
    (y, mo, d, h, mi, s)
}

fn is_leap(year: u64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_year(year: u64) -> u64 {
    match is_leap(year) {
        true => 366,
        false => 365,
    }
}

fn feb_days(year: u64) -> u64 {
    match is_leap(year) {
        true => 29,
        false => 28,
    }
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let diy = days_in_year(year);
        if days < diy { break; }
        days -= diy;
        year += 1;
    }
    let month_days: [u64; 12] = [31, feb_days(year), 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for md in &month_days {
        if days < *md { break; }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_rel_path_for_sanitizes_slashes() {
        let result = file_rel_path_for("PAY-4421", "02-features");
        assert_eq!(result, "02-features/PAY-4421.md");
    }

    #[test]
    fn file_rel_path_for_replaces_spaces() {
        let result = file_rel_path_for("PAY 123", "02-features");
        assert_eq!(result, "02-features/PAY_123.md");
    }

    #[test]
    fn file_rel_path_for_uses_custom_dest_dir() {
        let result = file_rel_path_for("JIRA-99", "03-epics");
        assert_eq!(result, "03-epics/JIRA-99.md");
    }

    #[test]
    fn dest_dir_from_routing_returns_default_when_no_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = dest_dir_from_routing(tmp.path());
        assert_eq!(result, DEFAULT_DEST_DIR);
    }

    #[test]
    fn dest_dir_from_routing_reads_custom_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let routing_path = tmp.path().join(ROUTING_CONFIG_FILE);
        std::fs::write(&routing_path, "tracker_import: 05-custom\n").unwrap();
        let result = dest_dir_from_routing(tmp.path());
        assert_eq!(result, "05-custom");
    }

    #[test]
    fn synced_at_now_produces_iso8601_format() {
        let ts = synced_at_now();
        assert!(ts.contains('T'), "must have T separator");
        assert!(ts.ends_with('Z'), "must end with Z");
        assert_eq!(ts.len(), 20, "must be exactly 20 chars: YYYY-MM-DDTHH:MM:SSZ");
    }
}
