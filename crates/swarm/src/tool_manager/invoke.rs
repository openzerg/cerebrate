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
    
    let mut cmd = if has_shell_nix {
        build_nix_command(tool_dir, entrypoint, &input_json, env_vars)
    } else {
        build_bwrap_command(tool_dir, entrypoint, &input_json, env_vars)
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

fn build_bwrap_command(
    tool_dir: &PathBuf,
    entrypoint: &str,
    input_json: &str,
    env_vars: &std::collections::HashMap<String, String>,
) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("bwrap");
    let tool_dir_str = tool_dir.display().to_string();
    
    cmd.args([
        "--ro-bind", "/nix/store", "/nix/store",
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
    
    for (key, value) in env_vars {
        cmd.args(["--setenv", key, value]);
    }
    
    cmd.args(["--setenv", "TOOL_INPUT", input_json]);
    
    if tool_dir.join("shell.nix").exists() {
        cmd.args([
            "--", "nix-shell", &tool_dir_str,
            "--run", &format!("{} '{}'", entrypoint, input_json.replace("'", "'\\''")),
        ]);
    } else {
        cmd.arg("--");
        cmd.args(entrypoint.split_whitespace());
        cmd.arg(input_json);
    }
    
    cmd
}

fn build_nix_command(
    tool_dir: &PathBuf,
    entrypoint: &str,
    input_json: &str,
    env_vars: &std::collections::HashMap<String, String>,
) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("nix-shell");
    
    cmd.arg(tool_dir)
       .arg("--run")
       .arg(&format!("{} '{}'", entrypoint, input_json.replace("'", "'\\''")));
    
    for (key, value) in env_vars {
        cmd.env(key, value);
    }
    
    cmd.env("TOOL_INPUT", input_json);
    cmd
}