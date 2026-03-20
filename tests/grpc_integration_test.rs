mod grpc_integration_tests {
    use std::sync::Arc;
    use cerebrate::AppState;
    use cerebrate::grpc::SwarmGrpcServer;
    use cerebrate::grpc::cerebrate::*;
    use cerebrate::grpc::cerebrate::swarm_service_server::SwarmService;
    use tonic::Request;

    fn create_test_server() -> SwarmGrpcServer {
        SwarmGrpcServer::new(Arc::new(AppState::new_test()))
    }

    #[tokio::test]
    async fn test_list_agents_empty() {
        let server = create_test_server();
        let response = server.list_agents(Request::new(Empty {})).await;
        assert!(response.is_ok());
        let agents = response.unwrap().into_inner();
        assert!(agents.agents.is_empty());
    }

    #[tokio::test]
    async fn test_get_agent_not_found() {
        let server = create_test_server();
        let response = server.get_agent(Request::new(GetAgentRequest { name: "nonexistent".to_string() })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_create_agent_unimplemented() {
        let server = create_test_server();
        let response = server.create_agent(Request::new(CreateAgentRequest { 
            name: "test-agent".to_string(),
            forgejo_username: None,
        })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_delete_agent() {
        let server = create_test_server();
        let response = server.delete_agent(Request::new(DeleteAgentRequest { name: "any-agent".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_enable_agent() {
        let server = create_test_server();
        let response = server.enable_agent(Request::new(EnableAgentRequest { name: "test-agent".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_disable_agent() {
        let server = create_test_server();
        let response = server.disable_agent(Request::new(DisableAgentRequest { name: "test-agent".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_list_providers_empty() {
        let server = create_test_server();
        let response = server.list_providers(Request::new(Empty {})).await;
        assert!(response.is_ok());
        let providers = response.unwrap().into_inner();
        assert!(providers.providers.is_empty());
    }

    #[tokio::test]
    async fn test_create_provider_unimplemented() {
        let server = create_test_server();
        let response = server.create_provider(Request::new(CreateProviderRequest {
            name: "OpenAI".to_string(),
            provider_type: "openai".to_string(),
            base_url: "https://api.openai.com".to_string(),
            api_key: "sk-test".to_string(),
        })).await;
        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_delete_provider() {
        let server = create_test_server();
        let response = server.delete_provider(Request::new(DeleteProviderRequest { id: "provider-1".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_enable_provider() {
        let server = create_test_server();
        let response = server.enable_provider(Request::new(EnableProviderRequest { id: "provider-1".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_disable_provider() {
        let server = create_test_server();
        let response = server.disable_provider(Request::new(DisableProviderRequest { id: "provider-1".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_list_models_empty() {
        let server = create_test_server();
        let response = server.list_models(Request::new(Empty {})).await;
        assert!(response.is_ok());
        let models = response.unwrap().into_inner();
        assert!(models.models.is_empty());
    }

    #[tokio::test]
    async fn test_create_model_unimplemented() {
        let server = create_test_server();
        let response = server.create_model(Request::new(CreateModelRequest {
            name: "GPT-4".to_string(),
            provider_id: "provider-1".to_string(),
            model_name: "gpt-4".to_string(),
        })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_delete_model() {
        let server = create_test_server();
        let response = server.delete_model(Request::new(DeleteModelRequest { id: "model-1".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_enable_model() {
        let server = create_test_server();
        let response = server.enable_model(Request::new(EnableModelRequest { id: "model-1".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_disable_model() {
        let server = create_test_server();
        let response = server.disable_model(Request::new(DisableModelRequest { id: "model-1".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_list_checkpoints_empty() {
        let server = create_test_server();
        let response = server.list_checkpoints(Request::new(ListCheckpointsRequest { agent: None })).await;
        assert!(response.is_ok());
        let checkpoints = response.unwrap().into_inner();
        assert!(checkpoints.checkpoints.is_empty());
    }

    #[tokio::test]
    async fn test_create_checkpoint_unimplemented() {
        let server = create_test_server();
        let response = server.create_checkpoint(Request::new(CreateCheckpointRequest {
            agent: "agent-1".to_string(),
            description: Some("Test checkpoint".to_string()),
        })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_delete_checkpoint() {
        let server = create_test_server();
        let response = server.delete_checkpoint(Request::new(DeleteCheckpointRequest { id: "checkpoint-1".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_rollback_checkpoint() {
        let server = create_test_server();
        let response = server.rollback_checkpoint(Request::new(RollbackCheckpointRequest {
            agent: "agent-1".to_string(),
            checkpoint_id: "checkpoint-1".to_string(),
        })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_clone_checkpoint_unimplemented() {
        let server = create_test_server();
        let response = server.clone_checkpoint(Request::new(CloneCheckpointRequest {
            id: "checkpoint-1".to_string(),
            new_name: "new-agent".to_string(),
        })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_list_skills_empty() {
        let server = create_test_server();
        let response = server.list_skills(Request::new(Empty {})).await;
        assert!(response.is_ok());
        let skills = response.unwrap().into_inner();
        assert!(skills.skills.is_empty());
    }

    #[tokio::test]
    async fn test_get_skill_not_found() {
        let server = create_test_server();
        let response = server.get_skill(Request::new(GetSkillRequest { slug: "nonexistent".to_string() })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_clone_skill_unimplemented() {
        let server = create_test_server();
        let response = server.clone_skill(Request::new(CloneSkillRequest {
            slug: "test-skill".to_string(),
            author_agent: "agent-1".to_string(),
            forgejo_repo: "org/skill".to_string(),
        })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_pull_skill_unimplemented() {
        let server = create_test_server();
        let response = server.pull_skill(Request::new(PullSkillRequest { slug: "test-skill".to_string() })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_delete_skill() {
        let server = create_test_server();
        let response = server.delete_skill(Request::new(DeleteSkillRequest { slug: "test-skill".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_list_tools_empty() {
        let server = create_test_server();
        let response = server.list_tools(Request::new(Empty {})).await;
        assert!(response.is_ok());
        let tools = response.unwrap().into_inner();
        assert!(tools.tools.is_empty());
    }

    #[tokio::test]
    async fn test_get_tool_not_found() {
        let server = create_test_server();
        let response = server.get_tool(Request::new(GetToolRequest { slug: "nonexistent".to_string() })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_clone_tool_unimplemented() {
        let server = create_test_server();
        let response = server.clone_tool(Request::new(CloneToolRequest {
            slug: "test-tool".to_string(),
            author_agent: "agent-1".to_string(),
            forgejo_repo: "org/tool".to_string(),
        })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_pull_tool_unimplemented() {
        let server = create_test_server();
        let response = server.pull_tool(Request::new(PullToolRequest { slug: "test-tool".to_string() })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_delete_tool() {
        let server = create_test_server();
        let response = server.delete_tool(Request::new(DeleteToolRequest { slug: "test-tool".to_string() })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_authorize_tool() {
        let server = create_test_server();
        let response = server.authorize_tool(Request::new(AuthorizeToolRequest {
            slug: "test-tool".to_string(),
            agent_name: "agent-1".to_string(),
        })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_revoke_tool() {
        let server = create_test_server();
        let response = server.revoke_tool(Request::new(RevokeToolRequest {
            slug: "test-tool".to_string(),
            agent_name: "agent-1".to_string(),
        })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_invoke_tool() {
        let server = create_test_server();
        let response = server.invoke_tool(Request::new(InvokeToolRequest {
            slug: "test-tool".to_string(),
            input_json: r#"{"query":"test"}"#.to_string(),
            caller: None,
        })).await;
        assert!(response.is_ok());
        let result = response.unwrap().into_inner();
        assert_eq!(result.output_json, "{}");
    }

    #[tokio::test]
    async fn test_list_tool_env_empty() {
        let server = create_test_server();
        let response = server.list_tool_env(Request::new(ListToolEnvRequest { slug: "test-tool".to_string() })).await;
        assert!(response.is_ok());
        let env = response.unwrap().into_inner();
        assert!(env.keys.is_empty());
    }

    #[tokio::test]
    async fn test_set_tool_env() {
        let server = create_test_server();
        let response = server.set_tool_env(Request::new(SetToolEnvRequest {
            slug: "test-tool".to_string(),
            key: "API_KEY".to_string(),
            value: "secret".to_string(),
        })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_delete_tool_env() {
        let server = create_test_server();
        let response = server.delete_tool_env(Request::new(DeleteToolEnvRequest {
            slug: "test-tool".to_string(),
            key: "API_KEY".to_string(),
        })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_get_stats() {
        let server = create_test_server();
        let response = server.get_stats(Request::new(Empty {})).await;
        assert!(response.is_ok());
        let stats = response.unwrap().into_inner();
        assert_eq!(stats.total_agents, 0);
        assert_eq!(stats.online_agents, 0);
        assert_eq!(stats.enabled_agents, 0);
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let server = create_test_server();
        let response = server.list_sessions(Request::new(ListSessionsRequest { agent: "agent-1".to_string() })).await;
        assert!(response.is_ok());
        let sessions = response.unwrap().into_inner();
        assert!(sessions.sessions.is_empty());
        assert_eq!(sessions.total, 0);
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let server = create_test_server();
        let response = server.get_session(Request::new(GetSessionRequest {
            agent: "agent-1".to_string(),
            id: "session-1".to_string(),
        })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_get_session_messages_empty() {
        let server = create_test_server();
        let response = server.get_session_messages(Request::new(GetSessionMessagesRequest {
            agent: "agent-1".to_string(),
            session_id: "session-1".to_string(),
            offset: None,
            limit: None,
        })).await;
        assert!(response.is_ok());
        let messages = response.unwrap().into_inner();
        assert!(messages.messages.is_empty());
        assert_eq!(messages.total, 0);
    }

    #[tokio::test]
    async fn test_send_session_chat() {
        let server = create_test_server();
        let response = server.send_session_chat(Request::new(SendSessionChatRequest {
            agent: "agent-1".to_string(),
            session_id: "session-1".to_string(),
            content: "Hello".to_string(),
        })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_interrupt_session() {
        let server = create_test_server();
        let response = server.interrupt_session(Request::new(InterruptSessionRequest {
            agent: "agent-1".to_string(),
            session_id: "session-1".to_string(),
            message: "Stop".to_string(),
        })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_get_session_context() {
        let server = create_test_server();
        let response = server.get_session_context(Request::new(GetSessionContextRequest {
            agent: "agent-1".to_string(),
            session_id: "session-1".to_string(),
        })).await;
        assert!(response.is_ok());
        let context = response.unwrap().into_inner();
        assert_eq!(context.context_json, "{}");
    }

    #[tokio::test]
    async fn test_list_processes_empty() {
        let server = create_test_server();
        let response = server.list_processes(Request::new(ListProcessesRequest { agent: "agent-1".to_string() })).await;
        assert!(response.is_ok());
        let processes = response.unwrap().into_inner();
        assert!(processes.processes.is_empty());
        assert_eq!(processes.total, 0);
    }

    #[tokio::test]
    async fn test_get_process_not_found() {
        let server = create_test_server();
        let response = server.get_process(Request::new(GetProcessRequest {
            agent: "agent-1".to_string(),
            id: "process-1".to_string(),
        })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_get_process_output() {
        let server = create_test_server();
        let response = server.get_process_output(Request::new(GetProcessOutputRequest {
            agent: "agent-1".to_string(),
            process_id: "process-1".to_string(),
            stream: None,
            offset: None,
            limit: None,
        })).await;
        assert!(response.is_ok());
        let output = response.unwrap().into_inner();
        assert_eq!(output.content, "");
        assert_eq!(output.total_size, 0);
    }

    #[tokio::test]
    async fn test_list_tasks_empty() {
        let server = create_test_server();
        let response = server.list_tasks(Request::new(ListTasksRequest { agent: "agent-1".to_string() })).await;
        assert!(response.is_ok());
        let tasks = response.unwrap().into_inner();
        assert!(tasks.tasks.is_empty());
        assert_eq!(tasks.total, 0);
    }

    #[tokio::test]
    async fn test_get_task_not_found() {
        let server = create_test_server();
        let response = server.get_task(Request::new(GetTaskRequest {
            agent: "agent-1".to_string(),
            id: "task-1".to_string(),
        })).await;
        assert!(response.is_err());
        let status = response.unwrap_err();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_list_activities_empty() {
        let server = create_test_server();
        let response = server.list_activities(Request::new(ListActivitiesRequest { agent: "agent-1".to_string() })).await;
        assert!(response.is_ok());
        let activities = response.unwrap().into_inner();
        assert!(activities.activities.is_empty());
        assert_eq!(activities.total, 0);
    }

    #[tokio::test]
    async fn test_send_message() {
        let server = create_test_server();
        let response = server.send_message(Request::new(SendMessageRequest {
            agent: "agent-1".to_string(),
            content: "Hello".to_string(),
        })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_send_remind() {
        let server = create_test_server();
        let response = server.send_remind(Request::new(SendRemindRequest {
            agent: "agent-1".to_string(),
            message: "Reminder".to_string(),
        })).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_list_builtin_tools_empty() {
        let server = create_test_server();
        let response = server.list_builtin_tools(Request::new(ListBuiltinToolsRequest { agent: "agent-1".to_string() })).await;
        assert!(response.is_ok());
        let tools = response.unwrap().into_inner();
        assert!(tools.tools.is_empty());
    }

    #[tokio::test]
    async fn test_execute_builtin_tool() {
        let server = create_test_server();
        let response = server.execute_builtin_tool(Request::new(ExecuteBuiltinToolRequest {
            agent: "agent-1".to_string(),
            tool_name: "test-tool".to_string(),
            args_json: r#"{}"#.to_string(),
            session_id: None,
        })).await;
        assert!(response.is_ok());
        let result = response.unwrap().into_inner();
        assert_eq!(result.title, "");
        assert_eq!(result.output, "");
        assert_eq!(result.metadata_json, "{}");
        assert_eq!(result.attachments_json, "[]");
        assert!(!result.truncated);
    }
}