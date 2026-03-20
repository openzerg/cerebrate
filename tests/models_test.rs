mod models_tests {
    use cerebrate::models::*;

    fn create_test_agent() -> Agent {
        Agent {
            enabled: true,
            container_ip: "192.168.200.10".to_string(),
            host_ip: "192.168.200.1".to_string(),
            forgejo_username: Some("agent-1".to_string()),
            internal_token: "token-123".to_string(),
            model_id: Some("gpt-4".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn create_test_provider() -> Provider {
        Provider {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            provider_type: ProviderType::Openai,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test".to_string(),
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn create_test_model() -> Model {
        Model {
            id: "gpt-4".to_string(),
            name: "GPT-4".to_string(),
            provider_id: "openai".to_string(),
            model_name: "gpt-4-turbo".to_string(),
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_agent_creation() {
        let agent = create_test_agent();

        assert!(agent.enabled);
        assert_eq!(agent.container_ip, "192.168.200.10");
        assert_eq!(agent.model_id, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_agent_fields() {
        let agent = create_test_agent();

        assert!(agent.enabled);
        assert!(!agent.container_ip.is_empty());
    }

    #[test]
    fn test_provider_creation() {
        let provider = create_test_provider();

        assert_eq!(provider.id, "openai");
        assert!(provider.enabled);
        assert_eq!(provider.provider_type, ProviderType::Openai);
    }

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(ProviderType::from_str("openai"), Some(ProviderType::Openai));
        assert_eq!(ProviderType::from_str("azure"), Some(ProviderType::Azure));
        assert_eq!(
            ProviderType::from_str("anthropic"),
            Some(ProviderType::Anthropic)
        );
        assert_eq!(
            ProviderType::from_str("deepseek"),
            Some(ProviderType::Deepseek)
        );
        assert_eq!(
            ProviderType::from_str("moonshot"),
            Some(ProviderType::Moonshot)
        );
        assert_eq!(ProviderType::from_str("zhipu"), Some(ProviderType::Zhipu));
        assert_eq!(ProviderType::from_str("custom"), Some(ProviderType::Custom));
        assert_eq!(ProviderType::from_str("unknown"), None);
    }

    #[test]
    fn test_provider_type_as_str() {
        assert_eq!(ProviderType::Openai.as_str(), "openai");
        assert_eq!(ProviderType::Azure.as_str(), "azure");
        assert_eq!(ProviderType::Anthropic.as_str(), "anthropic");
    }

    #[test]
    fn test_model_creation() {
        let model = create_test_model();

        assert_eq!(model.id, "gpt-4");
        assert!(model.enabled);
        assert_eq!(model.provider_id, "openai");
    }

    #[test]
    fn test_state_new() {
        let state = State::new();

        assert!(state.agents.is_empty());
        assert!(state.providers.is_empty());
        assert!(state.models.is_empty());
        assert!(state.forgejo_users.is_empty());
        assert!(state.skills.is_empty());
        assert!(state.tools.is_empty());
        assert!(state.admin_token.is_none());
        assert_eq!(state.version, "1.0");
    }

    #[test]
    fn test_state_default() {
        let state = State::default();

        assert!(state.agents.is_empty());
        assert_eq!(state.version, "");
    }

    #[test]
    fn test_defaults() {
        let defaults = Defaults {
            port: 8080,
            container_subnet_base: "192.168.200".to_string(),
            forgejo_url: "http://localhost:3000".to_string(),
            forgejo_token: "token".to_string(),
        };

        assert_eq!(defaults.port, 8080);
        assert_eq!(defaults.container_subnet_base, "192.168.200");
    }

    #[test]
    fn test_defaults_default_impl() {
        let defaults = Defaults::default();

        assert_eq!(defaults.port, 0);
    }

    #[test]
    fn test_forgejo_user() {
        let user = ForgejoUser {
            username: "agent-1".to_string(),
            password: "password123".to_string(),
            email: "agent-1@example.com".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(user.username, "agent-1");
        assert_eq!(user.email, "agent-1@example.com");
    }

    #[test]
    fn test_checkpoint_meta() {
        let meta = CheckpointMeta {
            id: "cp-123".to_string(),
            agent_name: "agent-1".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            description: "Test checkpoint".to_string(),
            snapshot_ref: "snap-123".to_string(),
        };

        assert_eq!(meta.id, "cp-123");
        assert_eq!(meta.agent_name, "agent-1");
    }

    #[test]
    fn test_tool() {
        let tool = Tool {
            slug: "test-tool".to_string(),
            name: "Test Tool".to_string(),
            version: "1.0.0".to_string(),
            description: "A test tool".to_string(),
            entrypoint: "python main.py".to_string(),
            author_agent: "agent-1".to_string(),
            forgejo_repo: "org/tool".to_string(),
            git_commit: "abc123".to_string(),
            input_schema: None,
            output_schema: None,
            allowed_agents: vec!["agent-1".to_string(), "agent-2".to_string()],
            enabled: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(tool.slug, "test-tool");
        assert_eq!(tool.allowed_agents.len(), 2);
    }

    #[test]
    fn test_skill() {
        let skill = Skill {
            slug: "test-skill".to_string(),
            name: "Test Skill".to_string(),
            version: "1.0.0".to_string(),
            description: "A test skill".to_string(),
            author_agent: "agent-1".to_string(),
            forgejo_repo: "org/skill".to_string(),
            git_commit: "def456".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert_eq!(skill.slug, "test-skill");
    }

    #[test]
    fn test_state_serialization() {
        let mut state = State::new();
        state.admin_token = Some("secret".to_string());

        let json = serde_json::to_string(&state).unwrap();
        let parsed: State = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.admin_token, Some("secret".to_string()));
    }

    #[test]
    fn test_agent_serialization() {
        let agent = create_test_agent();

        let json = serde_json::to_string(&agent).unwrap();
        let parsed: Agent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.container_ip, agent.container_ip);
        assert_eq!(parsed.enabled, agent.enabled);
    }

    #[test]
    fn test_provider_type_serialization() {
        let pt = ProviderType::Openai;
        let json = serde_json::to_string(&pt).unwrap();
        assert_eq!(json, "\"openai\"");

        let parsed: ProviderType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ProviderType::Openai);
    }

    #[test]
    fn test_empty_state_json() {
        let json = r#"{"version":"1.0","defaults":{"port":17531,"container_subnet_base":"192.168.200","forgejo_url":"http://localhost:3000","forgejo_token":""},"agents":{},"providers":{},"models":{},"forgejo_users":{},"skills":{},"tools":{},"admin_token":null}"#;

        let state: State = serde_json::from_str(json).unwrap();
        assert!(state.agents.is_empty());
    }

    #[test]
    fn test_caller_identity() {
        let admin = CallerIdentity::Admin;
        let agent = CallerIdentity::Agent("agent-1".to_string());

        match admin {
            CallerIdentity::Admin => assert!(true),
            _ => assert!(false),
        }

        match agent {
            CallerIdentity::Agent(name) => assert_eq!(name, "agent-1"),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_create_provider_request() {
        let req = CreateProviderRequest {
            name: "OpenAI".to_string(),
            provider_type: ProviderType::Openai,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test".to_string(),
        };

        assert_eq!(req.name, "OpenAI");
        assert_eq!(req.provider_type, ProviderType::Openai);
    }

    #[test]
    fn test_create_model_request() {
        let req = CreateModelRequest {
            name: "GPT-4".to_string(),
            provider_id: "openai".to_string(),
            model_name: "gpt-4-turbo".to_string(),
        };

        assert_eq!(req.name, "GPT-4");
        assert_eq!(req.provider_id, "openai");
    }
}
