use dotenvy::dotenv;
use std::env;
use std::sync::{Arc, Mutex};

use pks::boot_indexer::index_vaults_on_boot;
use pks::mcp_server::McpServer;
use pks::state::PrevalentState;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let _log_guard = pks::observability::init_logging(&pks::observability::log_config_from_env());

    let args: Vec<String> = env::args().collect();

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        eprintln!("Usage: pks <init|doctor|hook-post-commit|status|validate|refresh> [path] [--stdio|--daemon]");
        return;
    }

    if args.len() > 1 && !args[1].starts_with('-') {
        let cmd = pks::cli::parse_args(&args);
        let exit_code = pks::cli::run_command(cmd).await;
        std::process::exit(exit_code);
    }

    if args.contains(&"--stdio".to_string()) {
        run_stdio_server().await;
        return;
    }

    if args.contains(&"--daemon".to_string()) {
        #[cfg(unix)]
        { run_daemon_server().await; return; }
        #[cfg(not(unix))]
        { eprintln!("--daemon mode is not supported on Windows. Use --stdio for MCP."); return; }
    }

    run_http_server().await;
}

async fn run_http_server() {
    let state = Arc::new(Mutex::new(PrevalentState::default()));
    let port = McpServer::port_from_env();
    let server = McpServer::new(port);
    let ct = server.cancellation_token();
    index_vaults_on_boot(Arc::clone(&state)).await;
    let state_for_shutdown = Arc::clone(&state);
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        ct.cancel();
    });
    #[cfg(unix)]
    setup_unix_sigterm(server.cancellation_token());
    tracing::info!("PKS HTTP server starting on port {port}...");
    let _ = tokio::spawn(server.run(Arc::clone(&state))).await;
    let guard = state_for_shutdown.lock().unwrap();
    if let Err(e) = guard.save_all_snapshots() {
        tracing::error!("Failed to save snapshots: {e}");
    }
}

#[cfg(unix)]
fn setup_unix_sigterm(ct: tokio_util::sync::CancellationToken) {
    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        sigterm.recv().await;
        tracing::info!("Shutdown signal received (SIGTERM)...");
        ct.cancel();
    });
}

async fn run_stdio_server() {
    use rmcp::ServiceExt;
    let state = Arc::new(Mutex::new(PrevalentState::default()));
    let state_for_indexing = Arc::clone(&state);
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        index_vaults_on_boot(state_for_indexing).await;
    });
    let handler = pks::mcp_server::PksHandler::new(state);
    let transport = rmcp::transport::io::stdio();
    match handler.serve(transport).await {
        Err(e) => eprintln!("stdio init error: {e}"),
        Ok(server) => {
            let _ = server.waiting().await;
        }
    }
}

#[cfg(unix)]
async fn run_daemon_server() {
    let pid_path = pks::daemon::pid_path();
    let lock_file = match pks::daemon::acquire_pid_lock(&pid_path) {
        Some(f) => f,
        None => {
            tracing::warn!("Another daemon process holds the lock, exiting");
            return;
        }
    };
    let state = Arc::new(Mutex::new(PrevalentState::default()));
    let state_for_indexing = Arc::clone(&state);
    tokio::spawn(async move {
        index_vaults_on_boot(state_for_indexing).await;
    });
    setup_ctrlc_cleanup();
    let server = Arc::new(pks::ipc::IpcServer::new(Arc::clone(&state)));
    tracing::info!("PKS Daemon starting IPC socket...");
    server.accept_loop().await;
    let _ = std::fs::remove_file(pks::ipc::SOCKET_PATH);
    drop(lock_file);
}

fn setup_ctrlc_cleanup() {
    ctrlc::set_handler(move || {
        let _ = std::fs::remove_file(pks::ipc::SOCKET_PATH);
        let _ = std::fs::remove_file(pks::daemon::pid_path());
        std::process::exit(0);
    })
    .unwrap_or_else(|e| tracing::warn!("Failed to set Ctrl+C handler: {e}"));
}
