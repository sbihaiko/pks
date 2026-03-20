mod server;

pub use server::IpcServer;

use serde::{Deserialize, Serialize};

/// Path to the Unix Domain Socket
pub const SOCKET_PATH: &str = "/tmp/pks.sock";

/// IPC protocol version — bump when breaking changes are made
pub const IPC_VER: u32 = 3;

/// Commands sent from client to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "payload")]
pub enum PksCommand {
    Ping,
    Search {
        query: String,
        repo_id: Option<String>,
        top_n: usize,
    },
    ListVaults,
    Refresh {
        dry_run: bool,
    },
}

/// Responses sent from daemon to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", content = "data")]
pub enum PksResponse {
    Pong { ver: u32 },
    SearchResults { hits: Vec<SearchHit> },
    VaultList { vaults: Vec<String> },
    RefreshDone {
        added: Vec<String>,
        removed: Vec<String>,
        unchanged: Vec<String>,
    },
    Err { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub repo_id: String,
    pub file_path: String,
    pub score: f32,
    pub snippet: String,
}

/// Client for sending commands to the daemon
pub struct IpcClient;

impl IpcClient {
    /// Returns true if the daemon at `SOCKET_PATH` responds to Ping within 200ms
    pub async fn is_server_running() -> bool {
        Self::is_server_running_at(SOCKET_PATH).await
    }

    /// Returns true if the daemon at `socket_path` responds to Ping within 200ms
    pub async fn is_server_running_at(socket_path: &str) -> bool {
        use std::time::Duration;
        use tokio::time::timeout;
        let result = timeout(
            Duration::from_millis(200),
            Self::try_ping_at(socket_path),
        )
        .await;
        matches!(result, Ok(Ok(_)))
    }

    async fn try_ping_at(socket_path: &str) -> Result<(), String> {
        match Self::send_command_to(&PksCommand::Ping, socket_path).await? {
            PksResponse::Pong { .. } => Ok(()),
            _ => Err("unexpected response to Ping".to_string()),
        }
    }

    /// Send a command to the daemon at `SOCKET_PATH`
    pub async fn send_command(cmd: &PksCommand) -> Result<PksResponse, String> {
        Self::send_command_to(cmd, SOCKET_PATH).await
    }

    /// Send a command to the daemon at the given `socket_path`
    pub async fn send_command_to(cmd: &PksCommand, socket_path: &str) -> Result<PksResponse, String> {
        use std::time::Duration;
        use tokio::time::timeout;
        timeout(Duration::from_millis(5000), Self::do_send(cmd, socket_path))
            .await
            .map_err(|_| "daemon timeout".to_string())?
    }

    #[cfg(unix)]
    async fn do_send(cmd: &PksCommand, socket_path: &str) -> Result<PksResponse, String> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixStream;
        let mut stream = UnixStream::connect(socket_path)
            .await
            .map_err(|_| "daemon not running".to_string())?;
        let mut line = serde_json::to_string(cmd)
            .map_err(|e| format!("serialization error: {e}"))?;
        line.push('\n');
        stream.write_all(line.as_bytes()).await.map_err(|e| format!("write error: {e}"))?;
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.map_err(|e| format!("read error: {e}"))?;
        serde_json::from_str(response_line.trim())
            .map_err(|e| format!("deserialization error: {e}"))
    }

    #[cfg(not(unix))]
    async fn do_send(_cmd: &PksCommand, _socket_path: &str) -> Result<PksResponse, String> {
        Err("Unix Domain Sockets are only supported on Unix platforms".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pks_command_ping_serializes_correctly() {
        let cmd = PksCommand::Ping;
        let json = serde_json::to_string(&cmd).expect("serialize Ping");
        let back: PksCommand = serde_json::from_str(&json).expect("deserialize Ping");
        assert!(matches!(back, PksCommand::Ping));
        assert!(json.contains("\"Ping\""));
    }

    #[test]
    fn test_pks_command_search_serializes() {
        let cmd = PksCommand::Search {
            query: "hello world".to_string(),
            repo_id: Some("my-repo".to_string()),
            top_n: 5,
        };
        let json = serde_json::to_string(&cmd).expect("serialize Search");
        assert!(json.contains("hello world"));
        assert!(json.contains("my-repo"));
        assert!(json.contains("5"));
        let back: PksCommand = serde_json::from_str(&json).expect("deserialize Search");
        match back {
            PksCommand::Search { query, repo_id, top_n } => {
                assert_eq!(query, "hello world");
                assert_eq!(repo_id, Some("my-repo".to_string()));
                assert_eq!(top_n, 5);
            }
            _ => panic!("expected Search variant"),
        }
    }

    #[test]
    fn test_pks_response_err_deserializes() {
        let json = r#"{"status":"Err","data":{"message":"something went wrong"}}"#;
        let resp: PksResponse = serde_json::from_str(json).expect("deserialize Err response");
        match resp {
            PksResponse::Err { message } => assert_eq!(message, "something went wrong"),
            _ => panic!("expected Err variant"),
        }
    }

    #[tokio::test]
    async fn test_is_server_running_returns_false_when_no_daemon() {
        let _ = std::fs::remove_file(SOCKET_PATH);
        let running = IpcClient::is_server_running().await;
        assert!(!running, "should return false when socket does not exist");
    }
}
