use swarm::Result;
use crate::cli::DEFAULT_PORT;

pub async fn handle_status() -> Result<()> {
    println!("Zerg Swarm Service Status\n");
    println!("{:<20} {:<10} {}", "SERVICE", "STATUS", "INFO");
    println!("{}", "-".repeat(60));
    
    let hw = check_service_health(DEFAULT_PORT).await;
    let status = if hw.0 { "\x1b[32mrunning\x1b[0m" } else { "\x1b[31mstopped\x1b[0m" };
    println!("{:<20} {:<10} {}", "zerg-swarm", status, hw.1);
    
    Ok(())
}

async fn check_service_health(port: u16) -> (bool, String) {
    let url = format!("http://localhost:{}/api/health", port);
    match reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => (true, format!("port {}", port)),
        Ok(resp) => (false, format!("HTTP {}", resp.status())),
        Err(e) => (false, e.to_string()),
    }
}