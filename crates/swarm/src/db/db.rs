use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use crate::models::{Agent, Defaults, ForgejoUser, Provider, ProviderType, ApiKey, CreateProviderRequest, CreateApiKeyRequest};
use crate::{Result, Error};
use sha2::{Digest, Sha256};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agents (
    name TEXT PRIMARY KEY,
    enabled INTEGER DEFAULT 1,
    container_ip TEXT NOT NULL,
    host_ip TEXT NOT NULL,
    forgejo_username TEXT,
    internal_token TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS forgejo_users (
    username TEXT PRIMARY KEY,
    password TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    base_url TEXT NOT NULL,
    api_key TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    provider_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (provider_id) REFERENCES providers(id)
);

CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);
"#;

#[derive(sqlx::FromRow)]
struct ConfigRow {
    key: String,
    value: String,
}

#[derive(sqlx::FromRow)]
struct AgentRow {
    name: String,
    enabled: i32,
    container_ip: String,
    host_ip: String,
    forgejo_username: Option<String>,
    internal_token: String,
    created_at: String,
    updated_at: String,
}

impl From<AgentRow> for Agent {
    fn from(row: AgentRow) -> Self {
        Agent {
            name: row.name,
            enabled: row.enabled == 1,
            container_ip: row.container_ip,
            host_ip: row.host_ip,
            forgejo_username: row.forgejo_username,
            internal_token: row.internal_token,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ForgejoUserRow {
    username: String,
    password: String,
    email: String,
    created_at: String,
}

impl From<ForgejoUserRow> for ForgejoUser {
    fn from(row: ForgejoUserRow) -> Self {
        ForgejoUser {
            username: row.username,
            password: row.password,
            email: row.email,
            created_at: row.created_at,
        }
    }
}

impl From<ForgejoUser> for ForgejoUserRow {
    fn from(user: ForgejoUser) -> Self {
        ForgejoUserRow {
            username: user.username,
            password: user.password,
            email: user.email,
            created_at: user.created_at,
        }
    }
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(path: &std::path::Path) -> Result<Self> {
        let db_url = format!("sqlite:{}?mode=rwc", path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        sqlx::raw_sql(SCHEMA)
            .execute(&pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let db = Self { pool };
        db.init_defaults().await?;
        
        Ok(db)
    }

    async fn init_defaults(&self) -> Result<()> {
        let defaults = Defaults::default();
        self.set_config_if_missing("port", &defaults.port.to_string()).await?;
        self.set_config_if_missing("container_subnet_base", &defaults.container_subnet_base).await?;
        self.set_config_if_missing("forgejo_url", &defaults.forgejo_url).await?;
        self.set_config_if_missing("forgejo_token", &defaults.forgejo_token).await?;
        Ok(())
    }

    async fn set_config_if_missing(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO config (key, value) VALUES (?, ?)"
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn set_config(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)"
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_config(&self, key: &str) -> Result<Option<String>> {
        let row: Option<ConfigRow> = sqlx::query_as(
            "SELECT key, value FROM config WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.value))
    }

    pub async fn get_defaults(&self) -> Defaults {
        Defaults {
            port: self.get_config("port").await
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(17531),
            container_subnet_base: self.get_config("container_subnet_base").await
                .ok()
                .flatten()
                .unwrap_or_else(|| "10.200".to_string()),
            forgejo_url: self.get_config("forgejo_url").await
                .ok()
                .flatten()
                .unwrap_or_else(|| "http://localhost:3000".to_string()),
            forgejo_token: self.get_config("forgejo_token").await
                .ok()
                .flatten()
                .unwrap_or_default(),
        }
    }

    pub async fn update_defaults(&self, defaults: &Defaults) -> Result<()> {
        self.set_config("port", &defaults.port.to_string()).await?;
        self.set_config("container_subnet_base", &defaults.container_subnet_base).await?;
        self.set_config("forgejo_url", &defaults.forgejo_url).await?;
        self.set_config("forgejo_token", &defaults.forgejo_token).await?;
        Ok(())
    }

    pub async fn list_agents(&self) -> Result<Vec<Agent>> {
        let rows: Vec<AgentRow> = sqlx::query_as(
            "SELECT * FROM agents ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_agent(&self, name: &str) -> Result<Option<Agent>> {
        let row: Option<AgentRow> = sqlx::query_as(
            "SELECT * FROM agents WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn create_agent(&self, agent: &Agent) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO agents (name, enabled, container_ip, host_ip, 
               forgejo_username, internal_token, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&agent.name)
        .bind(agent.enabled as i32)
        .bind(&agent.container_ip)
        .bind(&agent.host_ip)
        .bind(&agent.forgejo_username)
        .bind(&agent.internal_token)
        .bind(&agent.created_at)
        .bind(&agent.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_agent(&self, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM agents WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn update_agent_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE agents SET enabled = ?, updated_at = ? WHERE name = ?")
            .bind(enabled as i32)
            .bind(now)
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_next_agent_num(&self) -> Result<usize> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        
        Ok((count.0 + 1) as usize)
    }

    pub async fn get_agent_by_forgejo_user(&self, forgejo_username: &str) -> Result<Option<Agent>> {
        let row: Option<AgentRow> = sqlx::query_as(
            "SELECT * FROM agents WHERE forgejo_username = ?"
        )
        .bind(forgejo_username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_forgejo_users(&self) -> Result<Vec<ForgejoUser>> {
        let rows: Vec<ForgejoUserRow> = sqlx::query_as(
            "SELECT * FROM forgejo_users ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_forgejo_user(&self, username: &str) -> Result<Option<ForgejoUser>> {
        let row: Option<ForgejoUserRow> = sqlx::query_as(
            "SELECT * FROM forgejo_users WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn create_forgejo_user(&self, user: &ForgejoUser) -> Result<()> {
        sqlx::query(
            "INSERT INTO forgejo_users (username, password, email, created_at) VALUES (?, ?, ?, ?)"
        )
        .bind(&user.username)
        .bind(&user.password)
        .bind(&user.email)
        .bind(&user.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_forgejo_user(&self, username: &str) -> Result<()> {
        sqlx::query("DELETE FROM forgejo_users WHERE username = ?")
            .bind(username)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn bind_forgejo_user(&self, agent_name: &str, forgejo_username: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE agents SET forgejo_username = ?, updated_at = ? WHERE name = ?")
            .bind(forgejo_username)
            .bind(now)
            .bind(agent_name)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn unbind_forgejo_user(&self, agent_name: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE agents SET forgejo_username = NULL, updated_at = ? WHERE name = ?")
            .bind(now)
            .bind(agent_name)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn create_provider(&self, req: &CreateProviderRequest) -> Result<Provider> {
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        
        sqlx::query(
            "INSERT INTO providers (id, name, provider_type, base_url, api_key, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 1, ?, ?)"
        )
        .bind(&id)
        .bind(&req.name)
        .bind(req.provider_type.as_str())
        .bind(&req.base_url)
        .bind(&req.api_key)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Provider {
            id,
            name: req.name.clone(),
            provider_type: req.provider_type.clone(),
            base_url: req.base_url.clone(),
            api_key: req.api_key.clone(),
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn list_providers(&self) -> Result<Vec<Provider>> {
        let rows: Vec<(String, String, String, String, String, i32, String, String)> = sqlx::query_as(
            "SELECT id, name, provider_type, base_url, api_key, enabled, created_at, updated_at FROM providers ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|row| Provider {
            id: row.0,
            name: row.1,
            provider_type: ProviderType::from_str(&row.2).unwrap_or(ProviderType::Custom),
            base_url: row.3,
            api_key: row.4,
            enabled: row.5 != 0,
            created_at: row.6,
            updated_at: row.7,
        }).collect())
    }

    pub async fn get_provider(&self, id: &str) -> Result<Option<Provider>> {
        let row: Option<(String, String, String, String, String, i32, String, String)> = sqlx::query_as(
            "SELECT id, name, provider_type, base_url, api_key, enabled, created_at, updated_at FROM providers WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(|row| Provider {
            id: row.0,
            name: row.1,
            provider_type: ProviderType::from_str(&row.2).unwrap_or(ProviderType::Custom),
            base_url: row.3,
            api_key: row.4,
            enabled: row.5 != 0,
            created_at: row.6,
            updated_at: row.7,
        }))
    }

    pub async fn delete_provider(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM providers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn update_provider_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE providers SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled as i32)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn create_api_key(&self, req: &CreateApiKeyRequest) -> Result<(ApiKey, String)> {
        let provider = self.get_provider(&req.provider_id).await?
            .ok_or_else(|| Error::NotFound(format!("Provider not found: {}", req.provider_id)))?;
        
        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        let raw_key = format!("zsp-{}", uuid::Uuid::new_v4());
        let key_hash = Self::hash_key(&raw_key);

        sqlx::query(
            "INSERT INTO api_keys (id, name, key_hash, provider_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&req.name)
        .bind(&key_hash)
        .bind(&req.provider_id)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok((ApiKey {
            id,
            name: req.name.clone(),
            key_hash,
            provider_id: provider.id,
            created_at: now.clone(),
            updated_at: now,
        }, raw_key))
    }

    pub async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, name, key_hash, provider_id, created_at, updated_at FROM api_keys ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|row| ApiKey {
            id: row.0,
            name: row.1,
            key_hash: row.2,
            provider_id: row.3,
            created_at: row.4,
            updated_at: row.5,
        }).collect())
    }

    pub async fn get_api_key_by_hash(&self, key_hash: &str) -> Result<Option<(ApiKey, Provider)>> {
        let row: Option<(String, String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, name, key_hash, provider_id, created_at, updated_at FROM api_keys WHERE key_hash = ?"
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        match row {
            Some(row) => {
                let api_key = ApiKey {
                    id: row.0,
                    name: row.1,
                    key_hash: row.2.clone(),
                    provider_id: row.3.clone(),
                    created_at: row.4,
                    updated_at: row.5,
                };
                let provider = self.get_provider(&row.3).await?
                    .ok_or_else(|| Error::NotFound(format!("Provider not found: {}", row.3)))?;
                Ok(Some((api_key, provider)))
            }
            None => Ok(None),
        }
    }

    pub async fn delete_api_key(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM api_keys WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    pub fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}