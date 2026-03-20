mod protocol_types_tests {
    use cerebrate::protocol::*;
    use chrono::Utc;

    #[test]
    fn test_file_entry() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            path: "/home/user/test.txt".to_string(),
            is_dir: false,
            size: 1024,
            modified: Some(Utc::now()),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: FileEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "test.txt");
        assert_eq!(parsed.size, 1024);
        assert!(!parsed.is_dir);
    }

    #[test]
    fn test_file_entry_directory() {
        let entry = FileEntry {
            name: "mydir".to_string(),
            path: "/home/user/mydir".to_string(),
            is_dir: true,
            size: 0,
            modified: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: FileEntry = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_dir);
        assert!(parsed.modified.is_none());
    }

    #[test]
    fn test_file_tree_data() {
        let tree = FileTreeData {
            path: "/home/user".to_string(),
            entries: vec![
                FileEntry {
                    name: "file1.txt".to_string(),
                    path: "/home/user/file1.txt".to_string(),
                    is_dir: false,
                    size: 100,
                    modified: None,
                },
                FileEntry {
                    name: "subdir".to_string(),
                    path: "/home/user/subdir".to_string(),
                    is_dir: true,
                    size: 0,
                    modified: None,
                },
            ],
        };

        let json = serde_json::to_string(&tree).unwrap();
        let parsed: FileTreeData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.entries.len(), 2);
    }

    #[test]
    fn test_git_repo() {
        let repo = GitRepo {
            path: "/home/user/project".to_string(),
            remote_url: Some("https://github.com/user/repo.git".to_string()),
            branch: Some("main".to_string()),
            status: "clean".to_string(),
            ahead: 0,
            behind: 2,
        };

        let json = serde_json::to_string(&repo).unwrap();
        let parsed: GitRepo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.branch, Some("main".to_string()));
        assert_eq!(parsed.behind, 2);
    }

    #[test]
    fn test_priority_serde() {
        let priorities = vec![
            Priority::Low,
            Priority::Medium,
            Priority::High,
            Priority::Urgent,
        ];

        for p in priorities {
            let json = serde_json::to_string(&p).unwrap();
            let parsed: Priority = serde_json::from_str(&json).unwrap();
            assert_eq!(p, parsed);
        }
    }

    #[test]
    fn test_priority_values() {
        assert_eq!(Priority::Low, Priority::Low);
        assert_ne!(Priority::Low, Priority::High);
    }

    #[test]
    fn test_process_event_started() {
        let event = ProcessEvent::Started;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("started"));
    }

    #[test]
    fn test_process_event_completed() {
        let event = ProcessEvent::Completed { exit_code: 0 };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: ProcessEvent = serde_json::from_str(&json).unwrap();

        match parsed {
            ProcessEvent::Completed { exit_code } => assert_eq!(exit_code, 0),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_process_event_failed() {
        let event = ProcessEvent::Failed {
            error: "Something went wrong".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: ProcessEvent = serde_json::from_str(&json).unwrap();

        match parsed {
            ProcessEvent::Failed { error } => assert_eq!(error, "Something went wrong"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_resource_type_serde() {
        let types = vec![
            ResourceType::Cpu,
            ResourceType::Memory,
            ResourceType::Disk,
            ResourceType::Processes,
        ];

        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let parsed: ResourceType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, parsed);
        }
    }

    #[test]
    fn test_invoke_tool_response_success() {
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

    #[test]
    fn test_invoke_tool_response_error() {
        let resp = InvokeToolResponse {
            success: false,
            output: None,
            error: Some("Tool execution failed".to_string()),
        };

        let json = serde_json::to_string(&resp).unwrap();
        let parsed: InvokeToolResponse = serde_json::from_str(&json).unwrap();

        assert!(!parsed.success);
        assert!(parsed.error.is_some());
    }

    #[test]
    fn test_vm_skill_result() {
        let result = VmSkillResult {
            agent_name: "agent-1".to_string(),
            skill_id: "skill-123".to_string(),
            success: true,
            output: Some(serde_json::json!({"data": "value"})),
            error: None,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: VmSkillResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.agent_name, "agent-1");
        assert!(parsed.success);
    }

    #[test]
    fn test_host_execute_skill() {
        let exec = HostExecuteSkill {
            skill_id: "skill-123".to_string(),
            skill_name: "My Skill".to_string(),
            entrypoint: "python main.py".to_string(),
            skill_files: vec![("main.py".to_string(), "print('hello')".to_string())],
            input: serde_json::json!({"arg": "value"}),
            timeout_secs: 60,
        };

        let json = serde_json::to_string(&exec).unwrap();
        let parsed: HostExecuteSkill = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.skill_files.len(), 1);
        assert_eq!(parsed.timeout_secs, 60);
    }
}
