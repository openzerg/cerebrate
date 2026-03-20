use std::net::SocketAddr;
use cerebrate::Result;
use crate::cli::setup_logging;
use crate::state_init::init_state;
use cerebrate::api;

pub async fn handle_serve(data_dir: std::path::PathBuf) -> Result<()> {
    setup_logging();
    tracing::info!("Starting Cerebrate Manager...");
    
    let state = init_state(data_dir.clone()).await?;
    
    let sw = state.state_manager.load().await?;
    if let Some(ref token) = sw.admin_token {
        println!("\n========================================");
        println!("Admin Token: {}", token);
        println!("========================================\n");
    }
    
    let port = std::env::var("CEREBRATE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(17531);
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

    api::start_server(addr, state).await
}