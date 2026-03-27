use std::io::Write;
use std::process::Command;

use anyhow::{Context, Result, bail};

fn resolve_editor() -> String {
    std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string())
}

/// Write content to a temp file, open it in $VISUAL/$EDITOR, return the edited content.
/// The caller is responsible for suspending/resuming the TUI.
pub fn edit_in_external(content: &str, file_ext: &str) -> Result<String> {
    let dir = crate::dirs::cache_dir();
    std::fs::create_dir_all(&dir).context("Failed to create cache directory")?;

    let file_path = dir.join(format!("suggest.{file_ext}"));
    {
        let mut f = std::fs::File::create(&file_path)
            .context("Failed to create temp file for editor")?;
        f.write_all(content.as_bytes())?;
        f.flush()?;
    }

    let editor = resolve_editor();
    let parts: Vec<&str> = editor.split_whitespace().collect();
    let (cmd, args) = parts
        .split_first()
        .ok_or_else(|| anyhow::anyhow!("Empty $VISUAL/$EDITOR"))?;

    let status = Command::new(cmd)
        .args(args)
        .arg(&file_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to launch editor: {editor}"))?;

    if !status.success() {
        let _ = std::fs::remove_file(&file_path);
        bail!("Editor exited with non-zero status");
    }

    let result = std::fs::read_to_string(&file_path).context("Failed to read back edited file")?;
    let _ = std::fs::remove_file(&file_path);
    Ok(result)
}
