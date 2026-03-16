//! Integration tests for M10 singleton IPC behavior.
//! These tests exercise real IpcServer / IpcClient code paths — no mocks.
//! Each test uses a unique socket path to avoid race conditions during parallel execution.

#![cfg(unix)]

use pks::ipc::{IpcClient, IpcServer, PksCommand, PksResponse};
use pks::state::PrevalentState;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};

static SOCKET_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Generate a unique socket path per test invocation.
fn unique_socket() -> String {
    let id = SOCKET_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("/tmp/pks_test_{}_{}_{}.sock", std::process::id(), id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos())
}

#[tokio::test]
async fn test_is_server_running_false_when_no_socket() {
    let path = unique_socket();
    let _ = std::fs::remove_file(&path);
    assert!(!IpcClient::is_server_running_at(&path).await);
}

#[tokio::test]
async fn test_daemon_ping_responds_with_pong() {
    let path = unique_socket();
    let state = Arc::new(Mutex::new(PrevalentState::default()));
    let server = Arc::new(IpcServer::with_socket_path(Arc::clone(&state), path.clone()));
    let server_task = tokio::spawn(async move { server.accept_loop().await });

    // Give the server a moment to bind
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    assert!(IpcClient::is_server_running_at(&path).await, "daemon should be running");

    let resp = IpcClient::send_command_to(&PksCommand::Ping, &path).await;
    assert!(
        matches!(resp, Ok(PksResponse::Pong { .. })),
        "Ping must return Pong, got: {resp:?}"
    );

    server_task.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_list_vaults_returns_empty_when_no_repos() {
    let path = unique_socket();
    let state = Arc::new(Mutex::new(PrevalentState::default()));
    let server = Arc::new(IpcServer::with_socket_path(Arc::clone(&state), path.clone()));
    let server_task = tokio::spawn(async move { server.accept_loop().await });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let resp = IpcClient::send_command_to(&PksCommand::ListVaults, &path).await;
    match resp {
        Ok(PksResponse::VaultList { vaults }) => {
            assert!(vaults.is_empty(), "fresh state has no vaults");
        }
        other => panic!("expected VaultList, got: {other:?}"),
    }

    server_task.abort();
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_socket_exists_while_server_running() {
    let path = unique_socket();
    let state = Arc::new(Mutex::new(PrevalentState::default()));
    let server = Arc::new(IpcServer::with_socket_path(Arc::clone(&state), path.clone()));
    let server_task = tokio::spawn(async move { server.accept_loop().await });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(
        std::path::Path::new(&path).exists(),
        "socket must exist while accept_loop is running"
    );

    server_task.abort();
    // Note: aborting the task does not trigger cleanup — socket file remains.
    // This test verifies the socket WAS created (proving accept_loop bound successfully).
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let _ = std::fs::remove_file(&path);
}
