use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub async fn run_llm(engine: &str, prompt: &str) -> Result<String> {
    let bin = match engine {
        "claude" => "claude-cli",
        "gemini" => "gemini-cli",
        "cursor-agent" => "cursor-agent",
        "codex" => "codex-cli",
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown engine: {}. Use one of: 'claude', 'gemini', 'cursor-agent', 'codex'",
                engine
            ));
        }
    };

    // Check if binary exists
    let which_output = Command::new("which")
        .arg(bin)
        .output()
        .await
        .context("Failed to check if CLI tool exists")?;

    if !which_output.status.success() {
        return Err(anyhow::anyhow!(
            "CLI tool '{}' not found in PATH. Please install it first.",
            bin
        ));
    }

    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context(format!("Failed to spawn {}", bin))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .context("Failed to take stdin")?;
        stdin
            .write_all(prompt.as_bytes())
            .await
            .context("Failed to write to stdin")?;
        stdin.flush().await.context("Failed to flush stdin")?;
    }

    let output = child
        .wait_with_output()
        .await
        .context("Failed to wait for CLI process")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "CLI tool '{}' failed with exit code {}: {}",
            bin,
            output.status.code().unwrap_or(-1),
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.to_string())
}
