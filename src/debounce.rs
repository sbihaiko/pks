use std::collections::HashMap;
use std::time::Instant;

pub struct Debouncer {
    seen: HashMap<String, Instant>,
    window_ms: u64,
}

impl Debouncer {
    pub fn new_from_env() -> Self {
        let window_ms = std::env::var("PKS_DEBOUNCE_WINDOW_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500u64);
        Self { seen: HashMap::new(), window_ms }
    }

    pub fn is_duplicate_and_record(&mut self, key: &str) -> bool {
        let now = Instant::now();
        self.expire_stale_entries(now);
        if self.seen.contains_key(key) {
            return true;
        }
        self.seen.insert(key.to_string(), now);
        false
    }

    fn expire_stale_entries(&mut self, now: Instant) {
        let window = std::time::Duration::from_millis(self.window_ms);
        self.seen.retain(|_, recorded_at| now.duration_since(*recorded_at) < window);
    }

    pub fn make_dedup_key(commit_sha: Option<&str>, tree_hash: Option<&str>) -> String {
        match (commit_sha, tree_hash) {
            (Some(sha), _) => format!("sha:{sha}"),
            (None, Some(hash)) => format!("tree:{hash}"),
            (None, None) => "unkeyed".to_string(),
        }
    }
}
