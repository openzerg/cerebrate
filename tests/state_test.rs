mod state_tests {
    use cerebrate::state::StateManager;
    use cerebrate::models::{State, Agent, Provider, Model, ProviderType};
    use tempfile::TempDir;

    fn create_test_state_manager() -> (StateManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::new(temp_dir.path());
        (manager, temp_dir)
    }

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
            pylon_proxy_id: Some("openai-proxy".to_string()),
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

    #[tokio::test]
    async fn test_load_empty_state() {
        let (manager, _temp) = create_test_state_manager();
        
        let state = manager.load().await.unwrap();
        assert!(state.agents.is_empty());
        assert!(state.providers.is_empty());
        assert!(state.models.is_empty());
    }

    #[tokio::test]
    async fn test_save_and_load_state() {
        let (manager, _temp) = create_test_state_manager();
        
        let mut state = State::new();
        state.agents.insert("agent-1".to_string(), create_test_agent());
        
        manager.save(&state).await.unwrap();
        
        let loaded = manager.load().await.unwrap();
        assert_eq!(loaded.agents.len(), 1);
        assert!(loaded.agents.contains_key("agent-1"));
    }

    #[tokio::test]
    async fn test_update_state() {
        let (manager, _temp) = create_test_state_manager();
        
        let mut state = State::new();
        state.admin_token = Some("test-token".to_string());
        manager.save(&state).await.unwrap();
        
        state.admin_token = Some("new-token".to_string());
        manager.save(&state).await.unwrap();
        
        let loaded = manager.load().await.unwrap();
        assert_eq!(loaded.admin_token, Some("new-token".to_string()));
    }

    #[tokio::test]
    async fn test_state_with_providers() {
        let (manager, _temp) = create_test_state_manager();
        
        let mut state = State::new();
        state.providers.insert("openai".to_string(), create_test_provider());
        
        manager.save(&state).await.unwrap();
        
        let loaded = manager.load().await.unwrap();
        assert_eq!(loaded.providers.len(), 1);
        assert!(loaded.providers.contains_key("openai"));
    }

    #[tokio::test]
    async fn test_state_with_models() {
        let (manager, _temp) = create_test_state_manager();
        
        let mut state = State::new();
        state.models.insert("gpt-4".to_string(), create_test_model());
        
        manager.save(&state).await.unwrap();
        
        let loaded = manager.load().await.unwrap();
        assert_eq!(loaded.models.len(), 1);
    }

    #[tokio::test]
    async fn test_multiple_agents() {
        let (manager, _temp) = create_test_state_manager();
        
        let mut state = State::new();
        
        for i in 0..5 {
            let mut agent = create_test_agent();
            agent.container_ip = format!("192.168.200.{}", 10 + i);
            state.agents.insert(format!("agent-{}", i), agent);
        }
        
        manager.save(&state).await.unwrap();
        
        let loaded = manager.load().await.unwrap();
        assert_eq!(loaded.agents.len(), 5);
    }

    #[tokio::test]
    async fn test_state_default_values() {
        let state = State::new();
        
        assert!(state.agents.is_empty());
        assert!(state.providers.is_empty());
        assert!(state.models.is_empty());
        assert!(state.forgejo_users.is_empty());
        assert!(state.skills.is_empty());
        assert!(state.tools.is_empty());
        assert!(state.admin_token.is_none());
    }

    #[tokio::test]
    async fn test_state_serialization_roundtrip() {
        let (manager, _temp) = create_test_state_manager();
        
        let mut original = State::new();
        original.admin_token = Some("secret".to_string());
        original.agents.insert("a1".to_string(), create_test_agent());
        
        manager.save(&original).await.unwrap();
        let loaded = manager.load().await.unwrap();
        
        assert_eq!(loaded.admin_token, original.admin_token);
        assert_eq!(loaded.agents.len(), original.agents.len());
        
        let loaded_agent = loaded.agents.get("a1").unwrap();
        assert!(loaded_agent.enabled);
        assert_eq!(loaded_agent.container_ip, "192.168.200.10");
    }

    #[tokio::test]
    async fn test_state_version() {
        let (manager, _temp) = create_test_state_manager();
        
        let state = State::new();
        manager.save(&state).await.unwrap();
        
        let loaded = manager.load().await.unwrap();
        assert_eq!(loaded.version, "1.0");
    }

    #[tokio::test]
    async fn test_defaults_persistence() {
        let (manager, _temp) = create_test_state_manager();
        
        let mut state = State::new();
        state.defaults.port = 8080;
        state.defaults.container_subnet_base = "10.0.0".to_string();
        
        manager.save(&state).await.unwrap();
        
        let loaded = manager.load().await.unwrap();
        assert_eq!(loaded.defaults.port, 8080);
        assert_eq!(loaded.defaults.container_subnet_base, "10.0.0");
    }
}