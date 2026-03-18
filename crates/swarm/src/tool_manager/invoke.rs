use crate::error::{Error, Result};
use crate::models::InvokeToolResponse;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn execute(
    tool_dir: &PathBuf,
    entrypoint: &str,
    input: &serde_json::Value,
    env_vars: &std::collections::HashMap<String, String>,
) -> Result<InvokeToolResponse> {
    let input_json = serde_json::to_string(input)?;
    let has_shell_nix = tool_dir.join("shell.nix").exists();
    let bwrap_path = find_bwrap();
    
    let mut cmd = if has_shell_nix {
        let (closure, shell_env) = get_shell_closure_and_env(tool_dir).await?;
        build_bwrap_command(tool_dir, entrypoint, &input_json, env_vars, &bwrap_path, &closure, Some(&shell_env))
    } else {
        build_bwrap_command(tool_dir, entrypoint, &input_json, env_vars, &bwrap_path, &[], None)
    };
    
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    
    let mut child = cmd.spawn().map_err(|e| Error::Io(e))?;
    
    let stdout = child.stdout.take().ok_or_else(|| Error::Io(std::io::Error::new(
        std::io::ErrorKind::Other, "Failed to capture stdout",
    )))?;
    let stderr = child.stderr.take().ok_or_else(|| Error::Io(std::io::Error::new(
        std::io::ErrorKind::Other, "Failed to capture stderr",
    )))?;
    
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut stdout_eof = false;
    let mut stderr_eof = false;
    
    while !stdout_eof || !stderr_eof {
        tokio::select! {
            line = stdout_reader.next_line(), if !stdout_eof => {
                match line {
                    Ok(Some(l)) => stdout_lines.push(l),
                    Ok(None) => stdout_eof = true,
                    Err(_) => stdout_eof = true,
                }
            }
            line = stderr_reader.next_line(), if !stderr_eof => {
                match line {
                    Ok(Some(l)) => stderr_lines.push(l),
                    Ok(None) => stderr_eof = true,
                    Err(_) => stderr_eof = true,
                }
            }
        }
    }
    
    let status = child.wait().await.map_err(|e| Error::Io(e))?;
    
    if !status.success() {
        return Ok(InvokeToolResponse {
            success: false,
            output: None,
            error: Some(format!("Script failed: {}", stderr_lines.join("\n"))),
        });
    }
    
    let output_str = stdout_lines.join("\n");
    
    match serde_json::from_str::<serde_json::Value>(&output_str) {
        Ok(json) => Ok(InvokeToolResponse {
            success: true,
            output: Some(json),
            error: None,
        }),
        Err(_) => Ok(InvokeToolResponse {
            success: true,
            output: Some(serde_json::json!({"raw": output_str})),
            error: None,
        }),
    }
}

fn find_bwrap() -> PathBuf {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let bwrap = PathBuf::from(dir).join("bwrap");
            if bwrap.exists() {
                return bwrap;
            }
        }
    }
    PathBuf::from("bwrap")
}

#[derive(Debug, Clone)]
struct ShellEnv {
    path: String,
}

async fn get_shell_closure_and_env(tool_dir: &PathBuf) -> Result<(Vec<PathBuf>, ShellEnv)> {
    let drv_output = tokio::process::Command::new("nix-instantiate")
        .arg(tool_dir.join("shell.nix"))
        .output()
        .await
        .map_err(|e| Error::Io(e))?;
    
    if !drv_output.status.success() {
        return Err(Error::TaskFailed(format!(
            "Failed to instantiate shell.nix: {}",
            String::from_utf8_lossy(&drv_output.stderr)
        )));
    }
    
    let drv_path = String::from_utf8_lossy(&drv_output.stdout).trim().to_string();
    
    let realise_output = tokio::process::Command::new("nix-store")
        .args(["--realise", &drv_path])
        .output()
        .await
        .map_err(|e| Error::Io(e))?;
    
    if !realise_output.status.success() {
        return Err(Error::TaskFailed(format!(
            "Failed to realise derivation: {}",
            String::from_utf8_lossy(&realise_output.stderr)
        )));
    }
    
    let out_path = String::from_utf8_lossy(&realise_output.stdout).trim().to_string();
    
    let req_output = tokio::process::Command::new("nix-store")
        .args(["--query", "--requisites"])
        .arg(&out_path)
        .output()
        .await
        .map_err(|e| Error::Io(e))?;
    
    if !req_output.status.success() {
        return Err(Error::TaskFailed(format!(
            "Failed to query closure: {}",
            String::from_utf8_lossy(&req_output.stderr)
        )));
    }
    
    let closure: Vec<PathBuf> = String::from_utf8_lossy(&req_output.stdout)
        .lines()
        .filter(|line| {
            let path = line.trim();
            path.starts_with("/nix/store/") && !path.ends_with(".drv")
        })
        .map(|line| PathBuf::from(line.trim()))
        .collect();
    
    let path_output = tokio::process::Command::new("nix-shell")
        .args(["--pure", tool_dir.to_str().unwrap(), "--run", "echo $PATH"])
        .output()
        .await
        .map_err(|e| Error::Io(e))?;
    
    let shell_path = String::from_utf8_lossy(&path_output.stdout).trim().to_string();
    
    Ok((closure, ShellEnv { path: shell_path }))
}

fn build_bwrap_command(
    tool_dir: &PathBuf,
    entrypoint: &str,
    input_json: &str,
    env_vars: &std::collections::HashMap<String, String>,
    bwrap_path: &PathBuf,
    closure: &[PathBuf],
    shell_env: Option<&ShellEnv>,
) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(bwrap_path);
    let tool_dir_str = tool_dir.display().to_string();
    
    if closure.is_empty() {
        cmd.args(["--ro-bind", "/nix/store", "/nix/store"]);
    } else {
        for path in closure {
            let path_str = path.display().to_string();
            cmd.args(["--ro-bind", &path_str, &path_str]);
        }
        cmd.args(["--ro-bind", "/nix/store/.links", "/nix/store/.links"]);
    }
    
    cmd.args([
        "--ro-bind", &tool_dir_str, "/tool",
        "--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf",
        "--ro-bind", "/etc/hosts", "/etc/hosts",
        "--dev", "/dev",
        "--proc", "/proc",
        "--tmpfs", "/tmp",
        "--die-with-parent",
        "--chdir", "/tool",
        "--unshare-ipc",
        "--unshare-pid",
    ]);
    
    if let Some(env) = shell_env {
        cmd.args(["--setenv", "PATH", &env.path]);
    }
    
    for (key, value) in env_vars {
        cmd.args(["--setenv", key, value]);
    }
    
    cmd.arg("--");
    cmd.args(entrypoint.split_whitespace());
    cmd.arg(input_json);
    
    cmd
}