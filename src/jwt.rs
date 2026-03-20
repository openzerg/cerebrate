use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;

pub static JWT_SECRET: Lazy<Vec<u8>> = Lazy::new(|| {
    env::var("JWT_SECRET")
        .unwrap_or_else(|_| "openzerg-default-secret-change-in-production".to_string())
        .into_bytes()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub iss: String,
    pub sub: String,
    pub role: String,
    pub forgejo_user: Option<String>,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "agent")]
    Agent,
    #[serde(rename = "service")]
    Service,
}

impl Claims {
    pub fn new_admin(subject: &str) -> Self {
        Self {
            iss: "cerebrate.openzerg.local".to_string(),
            sub: subject.to_string(),
            role: "admin".to_string(),
            forgejo_user: None,
            iat: Utc::now().timestamp(),
            exp: (Utc::now() + Duration::hours(24)).timestamp(),
        }
    }

    pub fn new_agent(name: &str, forgejo_user: Option<&str>) -> Self {
        Self {
            iss: "cerebrate.openzerg.local".to_string(),
            sub: name.to_string(),
            role: "agent".to_string(),
            forgejo_user: forgejo_user.map(|s| s.to_string()),
            iat: Utc::now().timestamp(),
            exp: (Utc::now() + Duration::days(365)).timestamp(),
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    pub fn is_agent(&self) -> bool {
        self.role == "agent"
    }
}

pub fn encode_token(claims: &Claims) -> Result<String, jsonwebtoken::errors::Error> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(&JWT_SECRET),
    )
}

pub fn decode_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&JWT_SECRET),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(token_data.claims)
}

pub fn generate_jwt_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let secret: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_token() {
        let claims = Claims::new_admin("admin");
        let token = encode_token(&claims).unwrap();
        let decoded = decode_token(&token).unwrap();
        assert_eq!(decoded.sub, "admin");
        assert!(decoded.is_admin());
    }

    #[test]
    fn test_agent_token() {
        let claims = Claims::new_agent("agent-1", Some("agent-1-user"));
        let token = encode_token(&claims).unwrap();
        let decoded = decode_token(&token).unwrap();
        assert_eq!(decoded.sub, "agent-1");
        assert!(decoded.is_agent());
        assert_eq!(decoded.forgejo_user, Some("agent-1-user".to_string()));
    }
}
