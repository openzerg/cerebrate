#[cfg(test)]
mod tests {
    use super::super::*;
    use chrono::Utc;

    #[test]
    fn test_host_event_serialization() {
        let event = HostEvent {
            event_id: "evt-123".to_string(),
            event: AgentEventMessage::Query {
                query_id: "q-123".to_string(),
                question: "What is the weather?".to_string(),
            },
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("evt-123"));
        assert!(json.contains("query"));
    }

    #[test]
    fn test_host_event_deserialization() {
        let json = r#"{"event_id":"evt-456","event":{"type":"query","query_id":"q-456","question":"Hello"}}"#;
        let event: HostEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_id, "evt-456");
    }

    #[test]
    fn test_agent_event_tool_result() {
        let event = AgentEventMessage::ToolResult {
            tool_call_id: "tc-123".to_string(),
            result: serde_json::json!({"output": "success"}),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEventMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentEventMessage::ToolResult { tool_call_id, .. } => {
                assert_eq!(tool_call_id, "tc-123");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_agent_event_error() {
        let event = AgentEventMessage::Error {
            message: "Something went wrong".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEventMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentEventMessage::Error { message } => {
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_agent_event_log() {
        let event = AgentEventMessage::Log {
            level: "info".to_string(),
            message: "Processing request".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("info"));
        assert!(json.contains("Processing request"));
    }

    #[test]
    fn test_agent_event_status_update() {
        let event = AgentEventMessage::StatusUpdate {
            status: "running".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEventMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentEventMessage::StatusUpdate { status } => {
                assert_eq!(status, "running");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_agent_event_file_read() {
        let event = AgentEventMessage::FileRead {
            path: "/workspace/file.txt".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("/workspace/file.txt"));
    }

    #[test]
    fn test_agent_event_file_write() {
        let event = AgentEventMessage::FileWrite {
            path: "/workspace/output.txt".to_string(),
            content: "Hello, World!".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("/workspace/output.txt"));
        assert!(json.contains("Hello, World!"));
    }

    #[test]
    fn test_agent_event_file_list() {
        let event = AgentEventMessage::FileList {
            path: "/workspace".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEventMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentEventMessage::FileList { path } => {
                assert_eq!(path, "/workspace");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_agent_event_file_delete() {
        let event = AgentEventMessage::FileDelete {
            path: "/workspace/old.txt".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("/workspace/old.txt"));
    }

    #[test]
    fn test_agent_event_shell_exec() {
        let event = AgentEventMessage::ShellExec {
            command: "ls -la".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ls -la"));
    }

    #[test]
    fn test_agent_event_http_request() {
        let event = AgentEventMessage::HttpRequest {
            url: "https://api.example.com/data".to_string(),
            method: "GET".to_string(),
            headers: Some(serde_json::json!({"Authorization": "Bearer token"})),
            body: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("https://api.example.com/data"));
        assert!(json.contains("GET"));
    }

    #[test]
    fn test_agent_event_query_response() {
        let event = AgentEventMessage::QueryResponse {
            query_id: "q-789".to_string(),
            response: "The answer is 42".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEventMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentEventMessage::QueryResponse { query_id, response } => {
                assert_eq!(query_id, "q-789");
                assert_eq!(response, "The answer is 42");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_vm_heartbeat_serialization() {
        let msg = VmHeartbeat {
            agent_name: "agent-1".to_string(),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("agent-1"));
    }

    #[test]
    fn test_agent_status_serialization() {
        let status = AgentStatus {
            online: true,
            cpu_percent: 50.0,
            memory_used_mb: 1000,
            memory_total_mb: 8000,
            disk_used_gb: 100.0,
            disk_total_gb: 500.0,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("online"));
        assert!(json.contains("50.0"));
    }

    #[test]
    fn test_file_entry_serialization() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            path: "/tmp/test.txt".to_string(),
            is_dir: false,
            size: 100,
            modified: Some(Utc::now()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("test.txt"));
        assert!(json.contains("false"));
    }

    #[test]
    fn test_git_repo_serialization() {
        let repo = GitRepo {
            path: "/home/user/repo".to_string(),
            remote_url: Some("https://github.com/user/repo".to_string()),
            branch: Some("main".to_string()),
            status: "clean".to_string(),
            ahead: 0,
            behind: 0,
        };
        let json = serde_json::to_string(&repo).unwrap();
        assert!(json.contains("clean"));
    }

    #[test]
    fn test_priority_serialization() {
        let priority = Priority::High;
        let json = serde_json::to_string(&priority).unwrap();
        assert_eq!(json, "\"high\"");
    }

    #[test]
    fn test_priority_deserialization() {
        let json = "\"urgent\"";
        let priority: Priority = serde_json::from_str(json).unwrap();
        assert_eq!(priority, Priority::Urgent);
    }

    #[test]
    fn test_process_event_started() {
        let event = ProcessEvent::Started;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"started\"");
    }

    #[test]
    fn test_process_event_completed() {
        let event = ProcessEvent::Completed { exit_code: 0 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("completed"));
        assert!(json.contains("0"));
    }

    #[test]
    fn test_process_event_failed() {
        let event = ProcessEvent::Failed {
            error: "timeout".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("failed"));
        assert!(json.contains("timeout"));
    }

    #[test]
    fn test_resource_type_serialization() {
        let resource = ResourceType::Memory;
        let json = serde_json::to_string(&resource).unwrap();
        assert_eq!(json, "\"memory\"");
    }

    #[test]
    fn test_agent_event_message_interrupt() {
        let msg = AgentEventMessage::Interrupt {
            message: "stop".to_string(),
            target_session: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("interrupt"));
        assert!(json.contains("stop"));
    }

    #[test]
    fn test_agent_event_message_query() {
        let msg = AgentEventMessage::Query {
            query_id: "q1".to_string(),
            question: "What is this?".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("q1"));
    }

    #[test]
    fn test_vm_event_ack_serialization() {
        let ack = VmEventAck {
            event_id: "e1".to_string(),
            accepted: true,
            message: Some("OK".to_string()),
        };
        let json = serde_json::to_string(&ack).unwrap();
        assert!(json.contains("e1"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_message_to_json() {
        let msg = Message::VmHeartbeat(VmHeartbeat {
            agent_name: "agent-1".to_string(),
            timestamp: Utc::now(),
        });
        let json = msg.to_json().unwrap();
        assert!(json.contains("vm_heartbeat"));
    }

    #[test]
    fn test_message_from_json() {
        let json =
            r#"{"type":"vm_heartbeat","agent_name":"test","timestamp":"2024-01-01T00:00:00Z"}"#;
        let msg = Message::from_json(json).unwrap();
        match msg {
            Message::VmHeartbeat(hb) => assert_eq!(hb.agent_name, "test"),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_host_execute_task_serialization() {
        let task = HostExecuteTask {
            task_id: "t1".to_string(),
            command: "ls -la".to_string(),
            cwd: Some("/tmp".to_string()),
            env: Some(vec![("KEY".to_string(), "VALUE".to_string())]),
        };
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("t1"));
        assert!(json.contains("ls -la"));
    }

    #[test]
    fn test_invoke_tool_response_success() {
        let resp = InvokeToolResponse {
            success: true,
            output: Some(serde_json::json!({"result": "ok"})),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("true"));
    }

    #[test]
    fn test_invoke_tool_response_error() {
        let resp = InvokeToolResponse {
            success: false,
            output: None,
            error: Some("Something went wrong".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("false"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn test_agent_event_serialization() {
        let event = AgentEvent {
            event: AgentEventType::Connected,
            agent_name: "agent-1".to_string(),
            timestamp: Utc::now(),
            data: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("connected"));
        assert!(json.contains("agent-1"));
    }

    #[test]
    fn test_agent_event_type_variants() {
        assert_eq!(
            serde_json::to_string(&AgentEventType::Connected).unwrap(),
            "\"connected\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::Disconnected).unwrap(),
            "\"disconnected\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::Created).unwrap(),
            "\"created\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::Deleted).unwrap(),
            "\"deleted\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::Enabled).unwrap(),
            "\"enabled\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::Disabled).unwrap(),
            "\"disabled\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::ConfigApplying).unwrap(),
            "\"config_applying\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::ConfigApplied).unwrap(),
            "\"config_applied\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::ConfigError).unwrap(),
            "\"config_error\""
        );
    }
}
