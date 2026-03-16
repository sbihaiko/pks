use std::path::PathBuf;

/// Returns the path to the PID lockfile.
pub fn pid_path() -> PathBuf {
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(tmp).join("pks.pid")
}

/// Attempts to acquire an exclusive lock on the PID file.
/// Returns `Some(file)` on success, `None` if another process holds the lock.
pub fn acquire_pid_lock(pid_path: &std::path::Path) -> Option<std::fs::File> {
    use fs2::FileExt;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(pid_path)
        .ok()?;
    match file.try_lock_exclusive() {
        Ok(()) => Some(file),
        Err(_) => None,
    }
}

/// Attempts to auto-spawn the daemon process in the background.
/// Returns `Ok(())` if the child was spawned, `Err` on failure.
pub fn auto_spawn_daemon() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe).arg("--daemon").spawn()?;
    Ok(())
}

/// Waits for the daemon to become reachable using exponential backoff.
/// Returns `true` if the daemon responded within the timeout, `false` otherwise.
pub async fn wait_for_daemon() -> bool {
    use crate::ipc::IpcClient;
    let delays_ms: [u64; 6] = [50, 100, 200, 400, 800, 1600];
    for delay_ms in delays_ms {
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        if IpcClient::is_server_running().await {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_path_is_absolute() {
        let path = pid_path();
        assert!(path.is_absolute(), "PID path must be absolute");
        assert!(
            path.to_string_lossy().ends_with("pks.pid"),
            "PID path must end with pks.pid"
        );
    }

    #[test]
    fn acquire_pid_lock_succeeds_on_fresh_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.pid");
        let lock = acquire_pid_lock(&path);
        assert!(lock.is_some(), "should acquire lock on new file");
    }

    #[test]
    fn acquire_pid_lock_fails_when_already_locked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test2.pid");
        let _lock1 = acquire_pid_lock(&path).expect("first lock must succeed");
        let lock2 = acquire_pid_lock(&path);
        assert!(lock2.is_none(), "second lock attempt must fail");
    }
}
