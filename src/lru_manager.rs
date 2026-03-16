use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub enum RepoStatus {
    Hot,
    Cold,
    Hibernated,
}

pub struct RepoActivity {
    pub repo_id: String,
    pub status: RepoStatus,
    pub last_query_secs: u64,
    pub vector_count: usize,
}

pub struct LruMemoryManager {
    pub repos: HashMap<String, RepoActivity>,
    pub total_vectors: usize,
    pub max_vectors: usize,
    pub hibernate_days: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn parse_env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

impl LruMemoryManager {
    pub fn from_env() -> Self {
        Self {
            repos: HashMap::new(),
            total_vectors: 0,
            max_vectors: parse_env_u64("PKS_MAX_VECTORS", 500_000) as usize,
            hibernate_days: parse_env_u64("PKS_HIBERNATE_DAYS", 7),
        }
    }

    pub fn register_repo(&mut self, repo_id: &str, vector_count: usize) {
        let activity = RepoActivity {
            repo_id: repo_id.to_string(),
            status: RepoStatus::Hot,
            last_query_secs: now_secs(),
            vector_count,
        };
        self.total_vectors += vector_count;
        self.repos.insert(repo_id.to_string(), activity);
    }

    pub fn record_query(&mut self, repo_id: &str) {
        let Some(activity) = self.repos.get_mut(repo_id) else {
            return;
        };
        activity.last_query_secs = now_secs();
        activity.status = RepoStatus::Hot;
    }

    pub fn find_lru_repo(&self) -> Option<String> {
        self.repos
            .values()
            .filter(|a| matches!(a.status, RepoStatus::Hot | RepoStatus::Cold))
            .min_by_key(|a| a.last_query_secs)
            .map(|a| a.repo_id.clone())
    }

    pub fn repos_to_hibernate(&self, now_secs: u64) -> Vec<String> {
        let threshold = self.hibernate_days * 86400;
        self.repos
            .values()
            .filter(|a| {
                !matches!(a.status, RepoStatus::Hibernated)
                    && now_secs.saturating_sub(a.last_query_secs) > threshold
            })
            .map(|a| a.repo_id.clone())
            .collect()
    }

    pub fn evict_if_over_watermark(&mut self) -> Option<String> {
        if self.total_vectors <= self.max_vectors {
            return None;
        }
        let lru = self.find_lru_repo()?;
        let removed = self.repos.remove(&lru)?;
        self.total_vectors = self.total_vectors.saturating_sub(removed.vector_count);
        Some(lru)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager_with_limit(max: usize) -> LruMemoryManager {
        LruMemoryManager { repos: HashMap::new(), total_vectors: 0, max_vectors: max, hibernate_days: 7 }
    }

    fn insert_hot(mgr: &mut LruMemoryManager, id: &str, secs: u64, vcount: usize) {
        let a = RepoActivity { repo_id: id.to_string(), status: RepoStatus::Hot, last_query_secs: secs, vector_count: vcount };
        mgr.repos.insert(id.to_string(), a);
        mgr.total_vectors += vcount;
    }

    #[test]
    fn lru_evicts_least_recently_used_repo() {
        let mut mgr = make_manager_with_limit(10);
        insert_hot(&mut mgr, "repo-old", 1000, 5);
        insert_hot(&mut mgr, "repo-mid", 2000, 3);
        insert_hot(&mut mgr, "repo-new", 3000, 5);

        let evicted = mgr.evict_if_over_watermark();
        assert_eq!(evicted, Some("repo-old".to_string()));
    }

    #[test]
    fn hibernate_detects_inactive_repos() {
        let mut mgr = make_manager_with_limit(500_000);
        let stale_secs = 1_000_000u64;
        insert_hot(&mut mgr, "repo-stale", stale_secs, 10);

        let to_hibernate = mgr.repos_to_hibernate(stale_secs + 8 * 86400);
        assert!(to_hibernate.contains(&"repo-stale".to_string()));
    }

    #[test]
    fn active_repo_not_hibernated() {
        let mut mgr = make_manager_with_limit(500_000);
        let now = 1_700_000_000u64;
        insert_hot(&mut mgr, "repo-active", now - 3600, 5);

        let to_hibernate = mgr.repos_to_hibernate(now);
        assert!(to_hibernate.is_empty());
    }

    #[test]
    fn register_and_record_query_updates_status() {
        let mut mgr = make_manager_with_limit(500_000);
        mgr.register_repo("repo-x", 42);
        let before = mgr.repos["repo-x"].last_query_secs;
        mgr.record_query("repo-x");
        let after = mgr.repos["repo-x"].last_query_secs;

        assert!(after >= before);
        assert_eq!(mgr.total_vectors, 42);
        assert!(matches!(mgr.repos["repo-x"].status, RepoStatus::Hot));
    }

    #[test]
    fn from_env_reads_defaults() {
        std::env::remove_var("PKS_MAX_VECTORS");
        std::env::remove_var("PKS_HIBERNATE_DAYS");
        let mgr = LruMemoryManager::from_env();
        assert_eq!(mgr.max_vectors, 500_000);
        assert_eq!(mgr.hibernate_days, 7);
    }
}
