use anyhow::{Context, bail};
use tokio::io::AsyncWriteExt;

use super::LlmProvider;

pub struct ClaudeCliProvider {
    model: String,
}

impl ClaudeCliProvider {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

impl LlmProvider for ClaudeCliProvider {
    fn complete(
        &self,
        system: &str,
        prompt: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<String>> + Send>> {
        let model = self.model.clone();
        let system = system.to_string();
        let prompt = prompt.to_string();

        Box::pin(async move {
            let mut child = tokio::process::Command::new("claude")
                .args(["--print", "--bare", "--output-format", "text"])
                .args(["--model", &model])
                .args(["--system-prompt", &system])
                .args(["--tools", ""])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn claude CLI. Is it installed?")?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(prompt.as_bytes())
                    .await
                    .context("Failed to write prompt to claude stdin")?;
            }

            let output = child
                .wait_with_output()
                .await
                .context("Failed to read claude output")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("claude exited with {}: {}", output.status, stderr.trim());
            }

            let text = String::from_utf8(output.stdout)
                .context("claude output was not valid UTF-8")?
                .trim()
                .to_string();

            if text.is_empty() {
                bail!("claude returned empty response");
            }

            Ok(text)
        })
    }
}
