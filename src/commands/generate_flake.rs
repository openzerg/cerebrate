use cerebrate::{Result, agent_manager::AgentManager};
use crate::state_init::init_state;
use std::path::PathBuf;

pub async fn handle_generate_flake(
    data_dir: PathBuf,
    output: Option<PathBuf>,
    template: Option<PathBuf>,
    force: bool,
) -> Result<()> {
    let state = init_state(data_dir.clone()).await?;
    let sw = state.state_manager.load().await?;
    
    let output_dir = output.unwrap_or_else(|| data_dir.join("system"));
    
    println!("Generating system flake files to: {}", output_dir.display());
    
    tokio::fs::create_dir_all(&output_dir).await?;
    
    let manager = AgentManager::new(&output_dir);
    
    if let Some(template_dir) = &template {
        copy_template_files(template_dir, &output_dir, force).await?;
    } else {
        ensure_template_files(&output_dir).await?;
    }
    
    let config_yaml = output_dir.parent()
        .map(|p| p.join("config.yaml"))
        .unwrap_or_else(|| data_dir.join("config.yaml"));
    
    let state_json_content = serde_json::to_string_pretty(&sw)?;
    tokio::fs::write(data_dir.join("state.json"), &state_json_content).await?;
    
    let yaml_content = serde_yaml::to_string(&sw)?;
    tokio::fs::write(&config_yaml, &yaml_content).await?;
    
    println!("Generated files:");
    println!("  - {}/flake.nix", output_dir.display());
    println!("  - {}/configuration.nix", output_dir.display());
    println!("  - {}/hardware-configuration.nix (if exists)", output_dir.display());
    println!("  - {}", config_yaml.display());
    println!("  - {}/state.json", data_dir.display());
    
    println!("\nContainers managed by Incus:");
    for (name, agent) in &sw.agents {
        let status = if agent.enabled { "enabled" } else { "disabled" };
        println!("  - {} ({})", name, status);
    }
    
    println!("\nNote: Containers are now managed by Incus, not NixOS.");
    println!("Run 'cerebrate apply' to create/update Incus containers.");
    
    Ok(())
}

async fn copy_template_files(template_dir: &PathBuf, output_dir: &PathBuf, force: bool) -> Result<()> {
    for file in ["flake.nix", "configuration.nix", "hardware-configuration.nix"] {
        let src = template_dir.join(file);
        let dst = output_dir.join(file);
        
        if src.exists() {
            if dst.exists() && !force {
                println!("  Skipping {} (exists, use --force to overwrite)", file);
                continue;
            }
            tokio::fs::copy(&src, &dst).await?;
            println!("  Copied {} from template", file);
        }
    }
    Ok(())
}

async fn ensure_template_files(output_dir: &PathBuf) -> Result<()> {
    let flake_path = output_dir.join("flake.nix");
    if !flake_path.exists() {
        let flake_content = include_str!("../../templates/flake.nix");
        tokio::fs::write(&flake_path, flake_content).await?;
        println!("  Created default flake.nix");
    }
    
    let config_path = output_dir.join("configuration.nix");
    if !config_path.exists() {
        let config_content = include_str!("../../templates/configuration.nix");
        tokio::fs::write(&config_path, config_content).await?;
        println!("  Created default configuration.nix");
    }
    
    let hardware_path = output_dir.join("hardware-configuration.nix");
    if !hardware_path.exists() {
        println!("  Warning: hardware-configuration.nix not found");
        println!("  Please run 'nixos-generate-config' or copy from /etc/nixos/hardware-configuration.nix");
    }
    
    Ok(())
}