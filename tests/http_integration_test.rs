mod http_integration_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode, Method},
    };
    use tower::ServiceExt;
    use serde_json::json;
    use std::sync::Arc;
    use cerebrate::{AppState, api, State};
    use serial_test::serial;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn setup_test_state() -> std::path::PathBuf {
        let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let data_dir = std::path::PathBuf::from(format!("/tmp/test_cerebrate_{}", test_id));
        let _ = std::fs::create_dir_all(&data_dir);
        
        let state = State {
            version: "1.0".to_string(),
            defaults: Default::default(),
            agents: Default::default(),
            providers: Default::default(),
            models: Default::default(),
            forgejo_users: Default::default(),
            skills: Default::default(),
            tools: Default::default(),
            admin_token: Some("admin-token-123".to_string()),
        };
        
        let state_path = data_dir.join("state.json");
        let _ = std::fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap());
        data_dir
    }

    fn create_test_app() -> axum::Router {
        let data_dir = setup_test_state();
        let state = Arc::new(AppState::new_test_with_dir(data_dir));
        api::create_router(state)
    }

    async fn get_jwt_token(app: &axum::Router) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"token": "admin-token-123"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        resp["data"]["jwt"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_login_success() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"token": "admin-token-123"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(resp["success"].as_bool().unwrap());
        assert!(resp["data"]["jwt"].is_string());
        assert_eq!(resp["data"]["expires_in"], 86400);
    }

    #[tokio::test]
    async fn test_auth_login_invalid_token() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"token": "wrong-token"}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(!resp["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_auth_verify_valid() {
        let app = create_test_app();
        let jwt = get_jwt_token(&app).await;
        
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/auth/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(json!(jwt).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(resp["data"]["valid"].as_bool().unwrap());
        assert_eq!(resp["data"]["subject"], "admin");
        assert_eq!(resp["data"]["role"], "admin");
    }

    #[tokio::test]
    async fn test_auth_verify_invalid() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/auth/verify")
                    .header("content-type", "application/json")
                    .body(Body::from(json!("invalid-token").to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(!resp["data"]["valid"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_list_agents_empty() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(resp["success"].as_bool().unwrap());
        assert!(resp["data"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_providers_empty() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/llm/providers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(resp["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_list_models_empty() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/llm/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(resp["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_list_skills() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_tools() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tools")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stats_summary() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/stats/summary")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cors_headers() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header("Origin", "http://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.headers().contains_key("access-control-allow-origin"));
    }

    #[tokio::test]
    async fn test_404_for_unknown_route() {
        let app = create_test_app();
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/unknown/endpoint")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}