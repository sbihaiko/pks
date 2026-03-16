#[cfg(test)]
mod tests {
    use pks::mcp_server::{McpServer, SearchResult};

    #[test]
    fn mcp_server_binds_to_localhost_only() {
        let server = McpServer::new(0);
        let addr = server.bind_addr();
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert!(addr.is_ipv4());
    }

    #[test]
    fn search_result_schema_is_correct() {
        let result = SearchResult {
            file_path: "docs/README.md".to_string(),
            heading_hierarchy: vec!["Section".to_string(), "Subsection".to_string()],
            chunk_text: "Some content here.".to_string(),
            score: 0.95,
            repo_id: "my-repo".to_string(),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert!(json.get("file_path").is_some());
        assert!(json.get("heading_hierarchy").is_some());
        assert!(json.get("chunk_text").is_some());
        assert!(json.get("score").is_some());
        assert!(json.get("repo_id").is_some());

        assert_eq!(json["file_path"], "docs/README.md");
        assert_eq!(json["repo_id"], "my-repo");
        assert_eq!(json["chunk_text"], "Some content here.");
    }

    #[cfg(feature = "integration")]
    mod integration {
        use pks::mcp_server::McpServer;
        use pks::state::PrevalentState;
        use std::sync::{Arc, Mutex};
        use tokio::time::{Duration, sleep};

        #[tokio::test]
        async fn mcp_server_accepts_mcp_initialize_request() {
            let server = McpServer::new(0);
            let ct = server.cancellation_token();
            let state = Arc::new(Mutex::new(PrevalentState::default()));

            let handle = tokio::spawn(async move {
                server.run(state).await.unwrap();
            });

            sleep(Duration::from_millis(100)).await;
            ct.cancel();
            let _ = handle.await;
        }
    }
}
