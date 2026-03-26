use anyhow::{Context, Result, bail};
use tokio::process::Command;

use std::collections::HashMap;

use crate::types::{
    DiffFile, ExistingComment, FileStatus, GhFile, PrMetadata, PrReview, ReviewComment,
    ReviewEvent, ThreadInfo,
};

async fn run_gh(args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .await
        .context("Failed to run gh CLI — is it installed?")?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", format_api_error(&stdout, &stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn try_extract_json_error(text: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    let main_msg = v["message"].as_str().unwrap_or("");
    let detail = v["errors"].as_array().and_then(|arr| {
        arr.first().and_then(|e| {
            e.as_str()
                .map(String::from)
                .or_else(|| e["message"].as_str().map(String::from))
        })
    });
    if let Some(detail) = detail {
        return Some(detail);
    }
    if !main_msg.is_empty() {
        return Some(main_msg.to_string());
    }
    None
}

fn format_api_error(stdout: &str, stderr: &str) -> String {
    if let Some(msg) = try_extract_json_error(stdout) {
        return msg;
    }
    if let Some(msg) = try_extract_json_error(stderr) {
        return msg;
    }
    let clean_stderr = stderr
        .trim()
        .strip_prefix("gh: ")
        .unwrap_or(stderr.trim());
    let clean_stdout = stdout.trim();
    if !clean_stdout.is_empty() && clean_stdout != clean_stderr {
        format!("{clean_stderr}: {clean_stdout}")
    } else {
        clean_stderr.to_string()
    }
}

pub async fn get_current_user() -> Result<String> {
    let output = run_gh(&["api", "user", "--jq", ".login"]).await?;
    Ok(output.trim().to_string())
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", format_api_error(&stdout, &stderr));
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
            let side_str = match c.side {
                crate::types::Side::Left => "LEFT",
                crate::types::Side::Right => "RIGHT",
            };
            let mut obj = serde_json::json!({
                "path": c.path,
                "line": c.line,
                "side": side_str,
                "body": c.body
            });
            if let Some(sl) = c.start_line {
                obj["start_line"] = serde_json::json!(sl);
            }
            if let Some(ss) = c.start_side {
                obj["start_side"] = serde_json::json!(match ss {
                    crate::types::Side::Left => "LEFT",
                    crate::types::Side::Right => "RIGHT",
                });
            }
            obj
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", format_api_error(&stdout, &stderr));
    }

    Ok(())
}

async fn run_graphql(query: &str, variables: &serde_json::Value) -> Result<serde_json::Value> {
    let body = serde_json::json!({
        "query": query,
        "variables": variables
    });
    let json_str = serde_json::to_string(&body)?;

    let mut child = Command::new("gh")
        .args(["api", "graphql", "--input", "-"])
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", format_api_error(&stdout, &stderr));
    }

    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse GraphQL response")?;
    Ok(result)
}

/// Fetch review thread resolve status. Returns a map from root comment database ID
/// to ThreadInfo (thread node_id + is_resolved).
pub async fn fetch_review_threads(
    repo: &str,
    pr_number: u64,
) -> Result<HashMap<u64, ThreadInfo>> {
    let (owner, name) = repo
        .split_once('/')
        .context("Invalid repo format, expected owner/name")?;

    let query = r#"
        query($owner: String!, $name: String!, $pr: Int!, $cursor: String) {
            repository(owner: $owner, name: $name) {
                pullRequest(number: $pr) {
                    reviewThreads(first: 100, after: $cursor) {
                        pageInfo { hasNextPage endCursor }
                        nodes {
                            id
                            isResolved
                            comments(first: 1) {
                                nodes { databaseId }
                            }
                        }
                    }
                }
            }
        }
    "#;

    let mut thread_map = HashMap::new();
    let mut cursor: Option<String> = None;

    loop {
        let vars = serde_json::json!({
            "owner": owner,
            "name": name,
            "pr": pr_number,
            "cursor": cursor,
        });

        let result = run_graphql(query, &vars).await?;
        let threads = &result["data"]["repository"]["pullRequest"]["reviewThreads"];

        if let Some(nodes) = threads["nodes"].as_array() {
            for node in nodes {
                let thread_id = node["id"].as_str().unwrap_or_default().to_string();
                let is_resolved = node["isResolved"].as_bool().unwrap_or(false);
                if let Some(first_comment) = node["comments"]["nodes"].as_array().and_then(|a| a.first())
                    && let Some(db_id) = first_comment["databaseId"].as_u64()
                {
                    thread_map.insert(db_id, ThreadInfo {
                        thread_node_id: thread_id,
                        is_resolved,
                    });
                }
            }
        }

        let has_next = threads["pageInfo"]["hasNextPage"]
            .as_bool()
            .unwrap_or(false);
        if has_next {
            cursor = threads["pageInfo"]["endCursor"]
                .as_str()
                .map(String::from);
        } else {
            break;
        }
    }

    Ok(thread_map)
}

pub async fn resolve_review_thread(thread_node_id: &str) -> Result<()> {
    let query = r#"
        mutation($threadId: ID!) {
            resolveReviewThread(input: {threadId: $threadId}) {
                thread { id isResolved }
            }
        }
    "#;
    let vars = serde_json::json!({ "threadId": thread_node_id });
    run_graphql(query, &vars).await?;
    Ok(())
}

pub async fn unresolve_review_thread(thread_node_id: &str) -> Result<()> {
    let query = r#"
        mutation($threadId: ID!) {
            unresolveReviewThread(input: {threadId: $threadId}) {
                thread { id isResolved }
            }
        }
    "#;
    let vars = serde_json::json!({ "threadId": thread_node_id });
    run_graphql(query, &vars).await?;
    Ok(())
}

pub async fn fetch_pr_reviews(repo: &str, pr_number: u64) -> Result<Vec<PrReview>> {
    let url = format!("repos/{repo}/pulls/{pr_number}/reviews");
    let output = run_gh(&["api", &url, "--paginate"]).await?;
    serde_json::from_str(&output).context("Failed to parse PR reviews")
}

pub async fn apply_suggestion(
    repo: &str,
    path: &str,
    head_ref: &str,
    branch: &str,
    line_number: usize,
    suggestion: &str,
) -> Result<()> {
    let file_content = fetch_file_content(repo, path, head_ref).await?;
    let mut lines: Vec<&str> = file_content.lines().collect();

    if line_number == 0 || line_number > lines.len() {
        bail!("Line {line_number} is out of range (file has {} lines)", lines.len());
    }

    let suggestion_lines: Vec<&str> = suggestion.lines().collect();
    lines.splice((line_number - 1)..line_number, suggestion_lines);

    let new_content = lines.join("\n") + "\n";

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(new_content.as_bytes());

    let file_url = format!("repos/{repo}/contents/{path}?ref={head_ref}");
    let file_resp = run_gh(&["api", &file_url]).await?;
    let file_meta: serde_json::Value =
        serde_json::from_str(&file_resp).context("Failed to parse file metadata")?;
    let sha = file_meta["sha"]
        .as_str()
        .context("Missing file SHA")?
        .to_string();

    let update_url = format!("repos/{repo}/contents/{path}");
    let json_body = serde_json::json!({
        "message": format!("Apply suggestion to {path}"),
        "content": encoded,
        "sha": sha,
        "branch": branch,
    });
    let json_str = serde_json::to_string(&json_body)?;

    let mut child = Command::new("gh")
        .args(["api", &update_url, "-X", "PUT", "--input", "-"])
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", format_api_error(&stdout, &stderr));
    }

    Ok(())
}

pub async fn dismiss_review(
    repo: &str,
    pr_number: u64,
    review_id: u64,
    message: &str,
) -> Result<()> {
    let url = format!("repos/{repo}/pulls/{pr_number}/reviews/{review_id}/dismissals");
    let json_body = serde_json::json!({ "message": message });
    let json_str = serde_json::to_string(&json_body)?;

    let mut child = Command::new("gh")
        .args(["api", &url, "-X", "PUT", "--input", "-"])
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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", format_api_error(&stdout, &stderr));
    }

    Ok(())
}
