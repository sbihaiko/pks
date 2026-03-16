use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum RepoEvent {
    Registered(PathBuf),
    Purged(PathBuf),
}

pub struct RepoWatcher {
    vaults_dir: PathBuf,
    sender: mpsc::Sender<RepoEvent>,
}

impl RepoWatcher {
    pub fn new(vaults_dir: PathBuf, sender: mpsc::Sender<RepoEvent>) -> Self {
        Self { vaults_dir, sender }
    }

    pub fn vaults_dir_from_env() -> PathBuf {
        let default = format!(
            "{}/pks-vaults",
            env::var("HOME").unwrap_or_else(|_| ".".to_string())
        );
        PathBuf::from(env::var("PKS_VAULTS_DIR").unwrap_or(default))
    }

    pub fn is_git_repo(path: &Path) -> bool {
        path.is_dir() && path.join(".git").exists()
    }

    pub fn scan_existing_repos(&self) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(&self.vaults_dir) else {
            return Vec::new();
        };
        entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| Self::is_git_repo(p))
            .collect()
    }

    pub fn start_watching(self) -> notify::Result<RecommendedWatcher> {
        let sender = self.sender.clone();
        let vaults_dir = self.vaults_dir.clone();

        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            let Ok(event) = result else {
                return;
            };
            handle_event(&event, &sender, &vaults_dir);
        })?;

        watcher.watch(&self.vaults_dir, RecursiveMode::NonRecursive)?;
        Ok(watcher)
    }
}

fn handle_event(event: &Event, sender: &mpsc::Sender<RepoEvent>, vaults_dir: &Path) {
    match event.kind {
        EventKind::Create(_) => {
            for path in &event.paths {
                let candidate = resolve_candidate(path, vaults_dir);
                let Some(candidate) = candidate else {
                    continue;
                };
                if !RepoWatcher::is_git_repo(&candidate) {
                    continue;
                }
                let _ = sender.send(RepoEvent::Registered(candidate));
            }
        }
        EventKind::Remove(_) => {
            for path in &event.paths {
                let candidate = resolve_candidate(path, vaults_dir);
                let Some(candidate) = candidate else {
                    continue;
                };
                let _ = sender.send(RepoEvent::Purged(candidate));
            }
        }
        _ => {}
    }
}

fn resolve_candidate(path: &Path, vaults_dir: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    if parent == vaults_dir {
        return Some(path.to_path_buf());
    }
    let grandparent = parent.parent()?;
    if grandparent == vaults_dir {
        return Some(parent.to_path_buf());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn is_git_repo_detects_dot_git_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path().to_path_buf();
        assert!(!RepoWatcher::is_git_repo(&repo_path));
        std::fs::create_dir(repo_path.join(".git")).unwrap();
        assert!(RepoWatcher::is_git_repo(&repo_path));
    }

    #[test]
    fn scan_existing_repos_finds_git_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let vaults_dir = tmp.path().to_path_buf();

        let repo_a = vaults_dir.join("repo-a");
        std::fs::create_dir_all(repo_a.join(".git")).unwrap();

        let plain_dir = vaults_dir.join("plain");
        std::fs::create_dir(&plain_dir).unwrap();

        let (tx, _rx) = mpsc::channel();
        let watcher = RepoWatcher::new(vaults_dir, tx);
        let repos = watcher.scan_existing_repos();

        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0], repo_a);
    }

    #[test]
    fn vaults_dir_from_env_falls_back_to_home() {
        let backup = std::env::var("PKS_VAULTS_DIR").ok();
        std::env::remove_var("PKS_VAULTS_DIR");
        let path = RepoWatcher::vaults_dir_from_env();
        assert!(path.to_string_lossy().ends_with("pks-vaults"));
        if let Some(val) = backup {
            std::env::set_var("PKS_VAULTS_DIR", val);
        }
    }
}
