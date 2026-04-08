pub mod claude_cli;
pub mod prompt;

/// Provider-agnostic LLM interface. Text in, text out.
pub trait LlmProvider: Send + Sync {
    fn complete(
        &self,
        system: &str,
        prompt: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<String>> + Send>>;
}

/// Check whether the `claude` CLI is available on PATH.
pub async fn detect_claude_cli() -> bool {
    tokio::process::Command::new("claude")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .is_ok_and(|s| s.success())
}
