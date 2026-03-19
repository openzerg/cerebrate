use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::AppState;
use crate::api::skills::SkillInfo;
use crate::models::CreateSkillRequest;
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, state: Arc<AppState>) {
    let state_clone = state.clone();
    registry.register("skill.list", move |_params| {
        let state = state_clone.clone();
        async move {
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            let skills: Vec<SkillInfo> = sw.skills.values().cloned().map(SkillInfo::from).collect();
            Ok(serde_json::to_value(skills).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("skill.get", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'slug'"))?;
            
            let sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let skill = sw.skills.get(&p.slug)
                .ok_or_else(|| RpcError::not_found(&format!("Skill '{}'", p.slug)))?;
            
            Ok(serde_json::to_value(SkillInfo::from(skill.clone())).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("skill.clone", move |params| {
        let state = state_clone.clone();
        async move {
            let req: CreateSkillRequest = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid request body"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if sw.skills.contains_key(&req.slug) {
                return Err(RpcError::already_exists(&format!("Skill '{}'", req.slug)));
            }
            
            if !sw.agents.contains_key(&req.author_agent) {
                return Err(RpcError::not_found(&format!("Agent '{}'", req.author_agent)));
            }
            
            state.tool_manager.clone_skill(&req.slug, &req.forgejo_repo).await
                .map_err(|e| RpcError::internal_error(format!("Failed to clone skill: {}", e)))?;
            
            let metadata = state.tool_manager.parse_skill_md(&req.slug)
                .map_err(|e| {
                    let _ = state.tool_manager.delete_skill(&req.slug);
                    RpcError::internal_error(format!("Failed to parse SKILL.md: {}", e))
                })?;
            
            let git_commit = state.tool_manager.get_skill_git_commit(&req.slug).await
                .map_err(|e| {
                    let _ = state.tool_manager.delete_skill(&req.slug);
                    RpcError::internal_error(format!("Failed to get git commit: {}", e))
                })?;
            
            let now = chrono::Utc::now().to_rfc3339();
            let skill = crate::models::Skill {
                slug: req.slug.clone(),
                name: metadata.name,
                version: metadata.version,
                description: metadata.description,
                forgejo_repo: req.forgejo_repo,
                git_commit,
                author_agent: req.author_agent,
                created_at: now.clone(),
                updated_at: now,
            };
            
            sw.skills.insert(req.slug.clone(), skill.clone());
            state.state_manager.save(&sw).await
                .map_err(|e| {
                    let _ = state.tool_manager.delete_skill(&req.slug);
                    RpcError::internal_error(e.to_string())
                })?;
            
            Ok(serde_json::to_value(SkillInfo::from(skill)).unwrap())
        }
    }).await;

    let state_clone = state.clone();
    registry.register("skill.pull", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'slug'"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if !sw.skills.contains_key(&p.slug) {
                return Err(RpcError::not_found(&format!("Skill '{}'", p.slug)));
            }
            
            let new_commit = state.tool_manager.pull_skill(&p.slug).await
                .map_err(|e| RpcError::internal_error(format!("Failed to pull skill: {}", e)))?;
            
            let metadata = state.tool_manager.parse_skill_md(&p.slug)
                .map_err(|e| RpcError::internal_error(format!("Failed to parse SKILL.md: {}", e)))?;
            
            if let Some(s) = sw.skills.get_mut(&p.slug) {
                s.git_commit = new_commit;
                s.version = metadata.version;
                s.description = metadata.description;
                s.updated_at = chrono::Utc::now().to_rfc3339();
                let skill = s.clone();
                let _ = state.state_manager.save(&sw).await;
                return Ok(serde_json::to_value(SkillInfo::from(skill)).unwrap());
            }
            
            Err(RpcError::not_found(&format!("Skill '{}'", p.slug)))
        }
    }).await;

    let state_clone = state.clone();
    registry.register("skill.delete", move |params| {
        let state = state_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { slug: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'slug'"))?;
            
            let mut sw = state.state_manager.load().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            if sw.skills.remove(&p.slug).is_none() {
                return Err(RpcError::not_found(&format!("Skill '{}'", p.slug)));
            }
            
            state.state_manager.save(&sw).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let _ = state.tool_manager.delete_skill(&p.slug).await;
            
            Ok(serde_json::json!({"deleted": true}))
        }
    }).await;
}