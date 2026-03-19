mod protocol_compat {
    use swarm::protocol::*;

    fn assert_serializes<T: serde::Serialize + serde::de::DeserializeOwned>(value: T) {
        let json = serde_json::to_string(&value).unwrap();
        let _parsed: T = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_vm_connect_compat() {
        let connect = VmConnect {
            agent_name: "test-agent".to_string(),
            internal_token: "token123".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_serializes(connect);
    }

    #[test]
    fn test_vm_heartbeat_compat() {
        let heartbeat = VmHeartbeat {
            agent_name: "test-agent".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_serializes(heartbeat);
    }

    #[test]
    fn test_vm_status_report_compat() {
        let status = VmStatusReport {
            agent_name: "test-agent".to_string(),
            timestamp: chrono::Utc::now(),
            data: AgentStatus {
                online: true,
                cpu_percent: 50.0,
                memory_used_mb: 1000,
                memory_total_mb: 8000,
                disk_used_gb: 100.0,
                disk_total_gb: 500.0,
            },
        };
        assert_serializes(status);
    }

    #[test]
    fn test_vm_event_ack_compat() {
        let ack = VmEventAck {
            event_id: "evt-123".to_string(),
            accepted: true,
            message: Some("OK".to_string()),
        };
        assert_serializes(ack);
    }

    #[test]
    fn test_host_event_interrupt() {
        let event = HostEvent {
            event_id: "evt-1".to_string(),
            event: AgentEventMessage::Interrupt {
                message: "Stop now".to_string(),
                target_session: None,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("interrupt"));
        assert!(json.contains("Stop now"));

        let parsed: HostEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_id, "evt-1");
    }

    #[test]
    fn test_host_event_query() {
        let event = HostEvent {
            event_id: "evt-2".to_string(),
            event: AgentEventMessage::Query {
                query_id: "q-123".to_string(),
                question: "What is the status?".to_string(),
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("What is the status"));

        let parsed: HostEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_id, "evt-2");
    }

    #[test]
    fn test_host_event_message() {
        let event = HostEvent {
            event_id: "evt-3".to_string(),
            event: AgentEventMessage::Message {
                content: "Hello from manager".to_string(),
                from: "admin".to_string(),
            },
        };
        assert_serializes(event);
    }

    #[test]
    fn test_host_event_assign_task() {
        let event = HostEvent {
            event_id: "evt-4".to_string(),
            event: AgentEventMessage::AssignTask {
                task_id: "task-1".to_string(),
                title: "Build feature".to_string(),
                description: "Implement new feature".to_string(),
                priority: Priority::High,
                deadline: None,
                context: Some(serde_json::json!({"repo": "test"})),
            },
        };
        assert_serializes(event);
    }

    #[test]
    fn test_host_event_remind() {
        let event = HostEvent {
            event_id: "evt-5".to_string(),
            event: AgentEventMessage::Remind {
                id: "remind-1".to_string(),
                message: "Don't forget to commit".to_string(),
            },
        };
        assert_serializes(event);
    }

    #[test]
    fn test_host_event_config_update() {
        let event = HostEvent {
            event_id: "evt-6".to_string(),
            event: AgentEventMessage::ConfigUpdate {
                llm_base_url: Some("http://new-api:8080".to_string()),
                llm_api_key: None,
                llm_model: Some("gpt-4o".to_string()),
            },
        };
        assert_serializes(event);
    }

    #[test]
    fn test_process_event_serialization() {
        let started = ProcessEvent::Started;
        let completed = ProcessEvent::Completed { exit_code: 0 };
        let failed = ProcessEvent::Failed {
            error: "timeout".to_string(),
        };

        assert!(serde_json::to_string(&started).unwrap().contains("started"));
        assert!(serde_json::to_string(&completed)
            .unwrap()
            .contains("completed"));
        assert!(serde_json::to_string(&failed).unwrap().contains("failed"));
    }

    #[test]
    fn test_priority_serialization() {
        assert_eq!(serde_json::to_string(&Priority::Low).unwrap(), "\"low\"");
        assert_eq!(
            serde_json::to_string(&Priority::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(serde_json::to_string(&Priority::High).unwrap(), "\"high\"");
        assert_eq!(
            serde_json::to_string(&Priority::Urgent).unwrap(),
            "\"urgent\""
        );
    }

    #[test]
    fn test_resource_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ResourceType::Cpu).unwrap(),
            "\"cpu\""
        );
        assert_eq!(
            serde_json::to_string(&ResourceType::Memory).unwrap(),
            "\"memory\""
        );
        assert_eq!(
            serde_json::to_string(&ResourceType::Disk).unwrap(),
            "\"disk\""
        );
    }

    #[test]
    fn test_message_vm_heartbeat() {
        let msg = Message::VmHeartbeat(VmHeartbeat {
            agent_name: "agent-1".to_string(),
            timestamp: chrono::Utc::now(),
        });

        let json = msg.to_json().unwrap();
        assert!(json.contains("vm_heartbeat"));
        assert!(json.contains("agent-1"));

        let parsed = Message::from_json(&json).unwrap();
        match parsed {
            Message::VmHeartbeat(hb) => assert_eq!(hb.agent_name, "agent-1"),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_message_vm_connect() {
        let msg = Message::VmConnect(VmConnect {
            agent_name: "agent-1".to_string(),
            internal_token: "secret-token".to_string(),
            timestamp: chrono::Utc::now(),
        });

        let json = msg.to_json().unwrap();
        assert!(json.contains("vm_connect"));
        assert!(json.contains("secret-token"));

        let parsed = Message::from_json(&json).unwrap();
        match parsed {
            Message::VmConnect(c) => {
                assert_eq!(c.agent_name, "agent-1");
                assert_eq!(c.internal_token, "secret-token");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_message_vm_status_report() {
        let msg = Message::VmStatusReport(VmStatusReport {
            agent_name: "agent-1".to_string(),
            timestamp: chrono::Utc::now(),
            data: AgentStatus {
                online: true,
                cpu_percent: 75.5,
                memory_used_mb: 2048,
                memory_total_mb: 8192,
                disk_used_gb: 50.0,
                disk_total_gb: 200.0,
            },
        });

        let json = msg.to_json().unwrap();
        let parsed = Message::from_json(&json).unwrap();
        match parsed {
            Message::VmStatusReport(r) => {
                assert_eq!(r.agent_name, "agent-1");
                assert!(r.data.online);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_message_vm_event_ack() {
        let msg = Message::VmEventAck(VmEventAck {
            event_id: "evt-123".to_string(),
            accepted: true,
            message: Some("Processed".to_string()),
        });

        let json = msg.to_json().unwrap();
        let parsed = Message::from_json(&json).unwrap();
        match parsed {
            Message::VmEventAck(ack) => {
                assert_eq!(ack.event_id, "evt-123");
                assert!(ack.accepted);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_message_host_event() {
        let msg = Message::HostEvent(HostEvent {
            event_id: "evt-789".to_string(),
            event: AgentEventMessage::Interrupt {
                message: "Stop!".to_string(),
                target_session: Some("session-1".to_string()),
            },
        });

        let json = msg.to_json().unwrap();
        assert!(json.contains("host_event"));
        assert!(json.contains("interrupt"));
        assert!(json.contains("Stop!"));

        let parsed = Message::from_json(&json).unwrap();
        match parsed {
            Message::HostEvent(he) => {
                assert_eq!(he.event_id, "evt-789");
                match he.event {
                    AgentEventMessage::Interrupt {
                        message,
                        target_session,
                    } => {
                        assert_eq!(message, "Stop!");
                        assert_eq!(target_session, Some("session-1".to_string()));
                    }
                    _ => panic!("Wrong event type"),
                }
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_tool_result_event() {
        let event = AgentEventMessage::ToolResult {
            tool_call_id: "tc-123".to_string(),
            result: serde_json::json!({"output": "success"}),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEventMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentEventMessage::ToolResult {
                tool_call_id,
                result,
            } => {
                assert_eq!(tool_call_id, "tc-123");
                assert_eq!(result["output"], "success");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_error_event() {
        let event = AgentEventMessage::Error {
            message: "Something went wrong".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("Something went wrong"));
    }

    #[test]
    fn test_log_event() {
        let event = AgentEventMessage::Log {
            level: "info".to_string(),
            message: "Processing request".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("log"));
        assert!(json.contains("info"));
        assert!(json.contains("Processing request"));
    }

    #[test]
    fn test_query_response_event() {
        let event = AgentEventMessage::QueryResponse {
            query_id: "q-456".to_string(),
            response: "The answer is 42".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEventMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            AgentEventMessage::QueryResponse { query_id, response } => {
                assert_eq!(query_id, "q-456");
                assert_eq!(response, "The answer is 42");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_agent_status_fields() {
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
        assert!(json.contains("cpu_percent"));
        assert!(json.contains("memory_used_mb"));
        assert!(json.contains("disk_total_gb"));
    }

    #[test]
    fn test_agent_event_type_serialization() {
        assert_eq!(
            serde_json::to_string(&AgentEventType::Connected).unwrap(),
            "\"connected\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::Disconnected).unwrap(),
            "\"disconnected\""
        );
        assert_eq!(
            serde_json::to_string(&AgentEventType::StatusUpdate).unwrap(),
            "\"status_update\""
        );
    }

    #[test]
    fn test_file_entry() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            path: "/workspace/test.txt".to_string(),
            is_dir: false,
            size: 100,
            modified: Some(chrono::Utc::now()),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: FileEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test.txt");
        assert!(!parsed.is_dir);
    }

    #[test]
    fn test_git_repo() {
        let repo = GitRepo {
            path: "/home/user/repo".to_string(),
            remote_url: Some("https://github.com/user/repo".to_string()),
            branch: Some("main".to_string()),
            status: "clean".to_string(),
            ahead: 0,
            behind: 0,
        };

        let json = serde_json::to_string(&repo).unwrap();
        let parsed: GitRepo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, "/home/user/repo");
        assert_eq!(parsed.status, "clean");
    }

    #[test]
    fn test_invoke_tool_response() {
        let resp = InvokeToolResponse {
            success: true,
            output: Some(serde_json::json!({"result": "ok"})),
            error: None,
        };

        let json = serde_json::to_string(&resp).unwrap();
        let parsed: InvokeToolResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert!(parsed.output.is_some());
    }
}
