mod auth_middleware_tests {
    use axum::http::{header, HeaderMap, HeaderValue};

    fn extract_token_from_header(headers: &HeaderMap) -> Option<String> {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(|s| s.to_string())
    }

    #[test]
    fn test_extract_token_valid() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer my-token-123"),
        );

        let token = extract_token_from_header(&headers);
        assert_eq!(token, Some("my-token-123".to_string()));
    }

    #[test]
    fn test_extract_token_missing_header() {
        let headers = HeaderMap::new();
        let token = extract_token_from_header(&headers);
        assert!(token.is_none());
    }

    #[test]
    fn test_extract_token_wrong_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Basic dXNlcjpwYXNz"),
        );

        let token = extract_token_from_header(&headers);
        assert!(token.is_none());
    }

    #[test]
    fn test_extract_token_empty_value() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static(""));

        let token = extract_token_from_header(&headers);
        assert!(token.is_none());
    }

    #[test]
    fn test_extract_token_bearer_only() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer "));

        let token = extract_token_from_header(&headers);
        assert_eq!(token, Some("".to_string()));
    }

    #[test]
    fn test_extract_token_with_spaces() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token with spaces"),
        );

        let token = extract_token_from_header(&headers);
        assert_eq!(token, Some("token with spaces".to_string()));
    }

    #[test]
    fn test_extract_token_case_sensitive() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("bearer token123"),
        );

        let token = extract_token_from_header(&headers);
        assert!(token.is_none());
    }

    #[test]
    fn test_extract_token_long_token() {
        let mut headers = HeaderMap::new();
        let long_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let auth_value = format!("Bearer {}", long_token);
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&auth_value).unwrap(),
        );

        let token = extract_token_from_header(&headers);
        assert_eq!(token, Some(long_token.to_string()));
    }

    #[test]
    fn test_extract_token_special_chars() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token-_.~"),
        );

        let token = extract_token_from_header(&headers);
        assert_eq!(token, Some("token-_.~".to_string()));
    }
}
