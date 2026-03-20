mod jwt_tests {
    use cerebrate::jwt::{decode_token, encode_token, generate_jwt_secret, Claims};
    use std::env;

    fn setup() {
        env::set_var("JWT_SECRET", "test-secret-for-unit-tests-32bytes!!");
    }

    #[test]
    fn test_encode_decode_admin_token() {
        setup();
        let claims = Claims::new_admin("test-admin");
        let token = encode_token(&claims).expect("Failed to encode token");
        assert!(!token.is_empty());

        let decoded = decode_token(&token).expect("Failed to decode token");
        assert_eq!(decoded.sub, "test-admin");
        assert!(decoded.is_admin());
        assert!(!decoded.is_agent());
        assert_eq!(decoded.role, "admin");
        assert_eq!(decoded.iss, "cerebrate.openzerg.local");
    }

    #[test]
    fn test_encode_decode_agent_token() {
        setup();
        let claims = Claims::new_agent("agent-001", Some("forgejo-user"));
        let token = encode_token(&claims).expect("Failed to encode token");

        let decoded = decode_token(&token).expect("Failed to decode token");
        assert_eq!(decoded.sub, "agent-001");
        assert!(decoded.is_agent());
        assert!(!decoded.is_admin());
        assert_eq!(decoded.forgejo_user, Some("forgejo-user".to_string()));
    }

    #[test]
    fn test_agent_token_without_forgejo_user() {
        setup();
        let claims = Claims::new_agent("agent-002", None);
        let token = encode_token(&claims).expect("Failed to encode token");

        let decoded = decode_token(&token).expect("Failed to decode token");
        assert_eq!(decoded.forgejo_user, None);
    }

    #[test]
    fn test_invalid_token_rejected() {
        setup();
        let invalid_token = "invalid.token.here";
        let result = decode_token(invalid_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_modified_token_rejected() {
        setup();
        let claims = Claims::new_admin("admin");
        let mut token = encode_token(&claims).expect("Failed to encode token");

        token.push('x');

        let result = decode_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_jwt_secret() {
        let secret1 = generate_jwt_secret();
        let secret2 = generate_jwt_secret();

        assert_ne!(secret1, secret2, "Each generated secret should be unique");
        assert!(!secret1.is_empty());
        assert!(secret1.len() > 50, "Secret should be sufficiently long");
    }

    #[test]
    fn test_token_expiration() {
        setup();
        let mut claims = Claims::new_admin("admin");
        claims.exp = 0;

        let token = encode_token(&claims).expect("Failed to encode token");
        let result = decode_token(&token);

        assert!(result.is_err(), "Expired token should be rejected");
    }

    #[test]
    fn test_token_structure() {
        setup();
        let claims = Claims::new_admin("test-user");
        let token = encode_token(&claims).expect("Failed to encode token");

        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "JWT should have 3 parts: header, payload, signature"
        );
    }

    #[test]
    fn test_claims_serialization() {
        setup();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let claims = Claims {
            iss: "test-issuer".to_string(),
            sub: "test-subject".to_string(),
            role: "admin".to_string(),
            forgejo_user: Some("test-user".to_string()),
            iat: now,
            exp: now + 86400,
        };

        let token = encode_token(&claims).expect("Failed to encode");
        let decoded = decode_token(&token).expect("Failed to decode");

        assert_eq!(decoded.iss, "test-issuer");
        assert_eq!(decoded.sub, "test-subject");
        assert_eq!(decoded.role, "admin");
        assert_eq!(decoded.forgejo_user, Some("test-user".to_string()));
    }
}
