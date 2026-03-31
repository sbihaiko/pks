use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

const DEFAULT_COOLDOWN_SECS: u64 = 300; // 5 minutes
const COOLDOWN_ENV_VAR: &str = "PKS_CIRCUIT_BREAKER_COOLDOWN_SECS";

pub struct CircuitBreaker {
    offline_repos: HashMap<String, Instant>,
    cooldown: Duration,
}

impl CircuitBreaker {
    pub fn new(cooldown: Duration) -> Self {
        Self {
            offline_repos: HashMap::new(),
            cooldown,
        }
    }

    pub fn default_cooldown() -> Duration {
        let secs = std::env::var(COOLDOWN_ENV_VAR)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_COOLDOWN_SECS);
        Duration::from_secs(secs)
    }

    pub fn mark_offline(&mut self, repo_path: &Path) {
        let key = repo_path.to_string_lossy().into_owned();
        self.offline_repos.insert(key, Instant::now());
    }

    pub fn is_available(&self, repo_path: &Path) -> bool {
        let key = repo_path.to_string_lossy();
        match self.offline_repos.get(key.as_ref()) {
            None => true,
            Some(since) => since.elapsed() >= self.cooldown,
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(Self::default_cooldown())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn mark_offline_then_is_available_returns_false() {
        let mut cb = CircuitBreaker::new(Duration::from_secs(60));
        let repo = PathBuf::from("/tmp/test_repo");
        cb.mark_offline(&repo);
        assert!(!cb.is_available(&repo));
    }

    #[test]
    fn after_cooldown_expires_is_available_returns_true() {
        let mut cb = CircuitBreaker::new(Duration::from_millis(10));
        let repo = PathBuf::from("/tmp/test_repo_cooldown");
        cb.mark_offline(&repo);
        std::thread::sleep(Duration::from_millis(20));
        assert!(cb.is_available(&repo));
    }

    #[test]
    fn unknown_repo_is_always_available() {
        let cb = CircuitBreaker::new(Duration::from_secs(60));
        let repo = PathBuf::from("/tmp/unknown_repo");
        assert!(cb.is_available(&repo));
    }
}
