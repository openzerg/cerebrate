pub mod agent;
pub mod llm;
pub mod checkpoint;
pub mod skill;
pub mod tool;
pub mod stats;
pub mod forward;

use crate::rpc::registry::RpcRegistry;
use crate::AppState;
use std::sync::Arc;

pub async fn register_all_methods(registry: &RpcRegistry, state: Arc<AppState>) {
    agent::register(registry, state.clone()).await;
    llm::register(registry, state.clone()).await;
    checkpoint::register(registry, state.clone()).await;
    skill::register(registry, state.clone()).await;
    tool::register(registry, state.clone()).await;
    stats::register(registry, state.clone()).await;
    forward::register(registry, state).await;
}