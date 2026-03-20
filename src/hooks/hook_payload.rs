use serde::Deserialize;

/// Payload delivered to PostToolUse hooks by Claude Code.
#[derive(Debug, Deserialize)]
pub struct PostToolUsePayload {
    pub session_id: String,
    pub cwd: String,
    pub tool_name: String,
    /// Raw tool input — structure varies by tool type.
    pub tool_input: serde_json::Value,
    pub tool_response: ToolResponse,
}

/// Success/failure indicator embedded in every PostToolUse payload.
#[derive(Debug, Deserialize)]
pub struct ToolResponse {
    pub success: bool,
}

/// Payload delivered to Stop hooks by Claude Code.
#[derive(Debug, Deserialize)]
pub struct StopPayload {
    pub session_id: String,
    pub cwd: String,
    pub stop_hook_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_post_tool_use_edit_example() {
        let json = r#"{
            "session_id": "abc-123",
            "cwd": "/tmp/test-project",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": "src/main.rs",
                "old_string": "fn old",
                "new_string": "fn new"
            },
            "tool_response": {
                "success": true
            }
        }"#;

        let payload: PostToolUsePayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.session_id, "abc-123");
        assert_eq!(payload.cwd, "/tmp/test-project");
        assert_eq!(payload.tool_name, "Edit");
        assert!(payload.tool_response.success);
        assert_eq!(payload.tool_input["file_path"], "src/main.rs");
    }

    #[test]
    fn deserialize_stop_payload() {
        let json = r#"{
            "session_id": "def-456",
            "cwd": "/tmp/workspace",
            "stop_hook_active": false
        }"#;

        let payload: StopPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.session_id, "def-456");
        assert_eq!(payload.cwd, "/tmp/workspace");
        assert!(!payload.stop_hook_active);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let json = r#"{
            "session_id": "ghi-789",
            "cwd": "/opt/project",
            "stop_hook_active": true,
            "future_field": "some_value",
            "another_unknown": 42
        }"#;

        let payload: StopPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.session_id, "ghi-789");
        assert!(payload.stop_hook_active);
    }
}
