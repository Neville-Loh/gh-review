use anyhow::{Context, Result, bail};
use tokio::process::Command;

use crate::types::{
    DiffFile, ExistingComment, FileStatus, GhFile, PrMetadata, ReviewComment, ReviewEvent,
};

async fn run_gh(args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .await
        .context("Failed to run gh CLI — is it installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn fetch_pr_metadata(repo: &str, pr_number: u64) -> Result<PrMetadata> {
    let url = format!("repos/{repo}/pulls/{pr_number}");
    let output = run_gh(&["api", &url]).await?;
    serde_json::from_str(&output).context("Failed to parse PR metadata")
}

pub async fn fetch_pr_files(repo: &str, pr_number: u64) -> Result<Vec<DiffFile>> {
    let url = format!("repos/{repo}/pulls/{pr_number}/files");
    let output = run_gh(&["api", &url, "--paginate"]).await?;
    let gh_files: Vec<GhFile> =
        serde_json::from_str(&output).context("Failed to parse PR files")?;

    let mut files = Vec::new();
    for f in gh_files {
        let hunks = if let Some(ref patch) = f.patch {
            crate::diff::parser::parse_patch(patch)
        } else {
            Vec::new()
        };

        files.push(DiffFile {
            path: f.filename,
            status: FileStatus::from_str(&f.status),
            additions: f.additions,
            deletions: f.deletions,
            hunks,
        });

        crate::highlight::highlight_file(files.last_mut().unwrap());
    }

    Ok(files)
}

pub async fn fetch_review_comments(repo: &str, pr_number: u64) -> Result<Vec<ExistingComment>> {
    let url = format!("repos/{repo}/pulls/{pr_number}/comments");
    let output = run_gh(&["api", &url, "--paginate"]).await?;
    serde_json::from_str(&output).context("Failed to parse review comments")
}

pub async fn fetch_file_content(repo: &str, path: &str, git_ref: &str) -> Result<String> {
    let url = format!("repos/{repo}/contents/{path}?ref={git_ref}");
    let output = run_gh(&["api", &url]).await?;
    let fc: crate::types::FileContent =
        serde_json::from_str(&output).context("Failed to parse file content")?;

    if fc.encoding == "base64" {
        use base64::Engine;
        let cleaned: String = fc.content.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&cleaned)
            .context("Failed to decode base64 file content")?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    } else {
        Ok(fc.content)
    }
}

pub async fn reply_to_comment(
    repo: &str,
    pr_number: u64,
    comment_id: u64,
    body: &str,
) -> Result<()> {
    let url = format!("repos/{repo}/pulls/{pr_number}/comments/{comment_id}/replies");
    let json_body = serde_json::json!({ "body": body });
    let json_str = serde_json::to_string(&json_body)?;

    let mut child = Command::new("gh")
        .args(["api", &url, "-X", "POST", "--input", "-"])
        .kill_on_drop(true)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    use tokio::io::AsyncWriteExt;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(json_str.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to post reply: {}", stderr.trim());
    }

    Ok(())
}

pub async fn submit_review(
    repo: &str,
    pr_number: u64,
    event: ReviewEvent,
    body: &str,
    comments: &[ReviewComment],
) -> Result<()> {
    let url = format!("repos/{repo}/pulls/{pr_number}/reviews");

    let comments_json: Vec<serde_json::Value> = comments
        .iter()
        .map(|c| {
            serde_json::json!({
                "path": c.path,
                "line": c.line,
                "side": match c.side {
                    crate::types::Side::Left => "LEFT",
                    crate::types::Side::Right => "RIGHT",
                },
                "body": c.body
            })
        })
        .collect();

    let json_body = serde_json::json!({
        "event": event.as_api_str(),
        "body": body,
        "comments": comments_json
    });

    let json_str = serde_json::to_string(&json_body)?;
    let mut child = Command::new("gh")
        .args(["api", &url, "-X", "POST", "--input", "-"])
        .kill_on_drop(true)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    use tokio::io::AsyncWriteExt;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(json_str.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh api POST reviews failed: {}", stderr.trim());
    }

    Ok(())
}
