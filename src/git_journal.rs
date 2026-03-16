use std::path::{Path, PathBuf};
use std::sync::mpsc;
use git2::DiffDelta;
use tracing::{info, warn};

use crate::state::{Branch, CommitSha, PipelineEvent, RawTransaction, RepoId, VectorClock};

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub repo_id: RepoId,
    pub repo_path: PathBuf,
    pub branch: Branch,
    pub commit_sha: CommitSha,
    pub tree_hash: Option<String>,
}

fn enqueue_commit_txn(info: &CommitInfo, tx: &mpsc::SyncSender<RawTransaction>) {
    let txn = RawTransaction {
        event: PipelineEvent::RepoRegistered {
            repo_id: info.repo_id.clone(),
            path: info.repo_path.clone(),
        },
        commit_sha: Some(info.commit_sha.clone()),
        tree_hash: info.tree_hash.clone(),
        branch: Some(info.branch.clone()),
        ingested_at: std::time::Instant::now(),
    };
    match tx.try_send(txn) {
        Ok(_) => info!(repo_id = %info.repo_id, sha = %info.commit_sha, "commit enqueued"),
        Err(_) => warn!(repo_id = %info.repo_id, "Fila 1 full — dropped (recoverable via Vector Clock)"),
    }
}

pub fn notify_commit(
    info: CommitInfo,
    tx: &mpsc::SyncSender<RawTransaction>,
    vector_clock: &mut VectorClock,
) -> CommitAction {
    let is_rebase = vector_clock.is_potential_rebase(&info.repo_id, &info.branch, &info.commit_sha);
    if is_rebase {
        warn!(repo_id = %info.repo_id, branch = %info.branch, new_sha = %info.commit_sha,
            "potential rebase detected — flagging for Drop & Rebuild");
        return CommitAction::TriggerRebuildFor(info);
    }
    vector_clock.update(&info.repo_id, &info.branch, &info.commit_sha);
    enqueue_commit_txn(&info, tx);
    CommitAction::Enqueued
}

#[derive(Debug)]
pub enum CommitAction {
    Enqueued,
    TriggerRebuildFor(CommitInfo),
}

fn md_path_from_delta(delta: DiffDelta) -> Option<String> {
    let path = delta.new_file().path()?;
    let is_md = path.extension().map(|e| e == "md").unwrap_or(false);
    if is_md { Some(path.to_string_lossy().into_owned()) } else { None }
}

fn collect_md_files_from_diff(diff: git2::Diff) -> Vec<String> {
    let mut md_files = Vec::new();
    diff.foreach(
        &mut |delta, _| { md_files.extend(md_path_from_delta(delta)); true },
        None, None, None,
    ).ok();
    md_files
}

fn resolve_trees<'a>(
    repo: &'a git2::Repository,
    from_sha: &str,
    to_sha: &str,
) -> Option<(git2::Tree<'a>, git2::Tree<'a>)> {
    let from = repo.revparse_single(from_sha).ok()?;
    let to = repo.revparse_single(to_sha).ok()?;
    Some((from.peel_to_tree().ok()?, to.peel_to_tree().ok()?))
}

pub fn get_changed_md_files(repo_path: &Path, from_sha: &str, to_sha: &str) -> Vec<String> {
    let repo = match git2::Repository::open(repo_path) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let (ft, tt) = match resolve_trees(&repo, from_sha, to_sha) {
        Some(pair) => pair,
        None => return vec![],
    };
    let diff = match repo.diff_tree_to_tree(Some(&ft), Some(&tt), None) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    collect_md_files_from_diff(diff)
}

pub fn get_repo_head_info(repo_path: &Path) -> Option<(CommitSha, Branch)> {
    let repo = git2::Repository::open(repo_path).ok()?;
    let head = repo.head().ok()?;
    let sha = head.peel_to_commit().ok()?.id().to_string();
    let branch = head.shorthand().unwrap_or("HEAD").to_string();
    Some((sha, branch))
}

fn send_ref_changes(
    event: notify::Event,
    event_tx: &mpsc::Sender<RefChange>,
    repo_path: &PathBuf,
) {
    use notify::EventKind;
    if !matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
        return;
    }
    for path in &event.paths {
        if let Some(branch) = path.file_name() {
            let _ = event_tx.send(RefChange {
                repo_path: repo_path.clone(),
                branch: branch.to_string_lossy().into_owned(),
            });
        }
    }
}

pub fn watch_git_refs(
    repo_path: &Path,
    event_tx: mpsc::Sender<RefChange>,
) -> notify::Result<notify::RecommendedWatcher> {
    use notify::{Event, Watcher};
    let refs_path = repo_path.join(".git/refs/heads");
    let repo_path_owned = repo_path.to_path_buf();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            send_ref_changes(event, &event_tx, &repo_path_owned);
        }
    })?;
    watcher.watch(&refs_path, notify::RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

#[derive(Debug)]
pub struct RefChange {
    pub repo_path: PathBuf,
    pub branch: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn notify_commit_enqueues_transaction() {
        let (tx, rx) = mpsc::sync_channel(10);
        let mut vc = VectorClock::default();
        let info = CommitInfo {
            repo_id: "repo-a".to_string(),
            repo_path: PathBuf::from("/tmp/repo-a"),
            branch: "main".to_string(),
            commit_sha: "abc123".to_string(),
            tree_hash: None,
        };
        let action = notify_commit(info, &tx, &mut vc);
        assert!(matches!(action, CommitAction::Enqueued));
        assert!(rx.try_recv().is_ok());
        assert_eq!(vc.get("repo-a", "main"), Some(&"abc123".to_string()));
    }

    #[test]
    fn notify_commit_detects_rebase_on_sha_change() {
        let (tx, _rx) = mpsc::sync_channel(10);
        let mut vc = VectorClock::default();
        vc.update("repo-b", "main", "original-sha");
        let info = CommitInfo {
            repo_id: "repo-b".to_string(),
            repo_path: PathBuf::from("/tmp/repo-b"),
            branch: "main".to_string(),
            commit_sha: "different-sha".to_string(),
            tree_hash: None,
        };
        let action = notify_commit(info, &tx, &mut vc);
        assert!(matches!(action, CommitAction::TriggerRebuildFor(_)));
    }

    #[test]
    fn get_changed_md_files_returns_empty_for_invalid_repo() {
        let files = get_changed_md_files(Path::new("/nonexistent"), "sha1", "sha2");
        assert!(files.is_empty());
    }
}
