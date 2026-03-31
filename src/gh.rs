use anyhow::{Context, Result, bail};
use tokio::process::Command;

use std::collections::HashMap;

use serde::Deserialize;

use crate::types::{
    DiffFile, ExistingComment, FileStatus, GhFile, PrMetadata, PrRef, PrReview, PrUser,
    ReviewComment, ReviewEvent, ReviewState, ReviewerInfo, ThreadInfo,
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
    let clean_stderr = stderr.trim().strip_prefix("gh: ").unwrap_or(stderr.trim());
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

/// Parse reviewer info from GraphQL `latestReviews` and `reviewRequests`.
fn parse_reviewers(pr: &serde_json::Value) -> Vec<ReviewerInfo> {
    let mut reviewers: Vec<ReviewerInfo> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Some(nodes) = pr["latestReviews"]["nodes"].as_array() {
        for node in nodes {
            let login = node["author"]["login"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let state = ReviewState::from_str(
                node["state"].as_str().unwrap_or(""),
            );
            if !login.is_empty() && seen.insert(login.clone()) {
                reviewers.push(ReviewerInfo { login, state });
            }
        }
    }

    if let Some(nodes) = pr["reviewRequests"]["nodes"].as_array() {
        for node in nodes {
            let reviewer = &node["requestedReviewer"];
            let login = reviewer["login"]
                .as_str()
                .or_else(|| reviewer["name"].as_str())
                .unwrap_or("")
                .to_string();
            if !login.is_empty() && seen.insert(login.clone()) {
                reviewers.push(ReviewerInfo {
                    login,
                    state: ReviewState::Pending,
                });
            }
        }
    }

    reviewers
}

/// Fetch PR metadata and review threads in a single GraphQL call.
/// Returns `(PrMetadata, HashMap<comment_db_id, ThreadInfo>)`.
pub async fn fetch_pr_data(
    repo: &str,
    pr_number: u64,
) -> Result<(PrMetadata, HashMap<u64, ThreadInfo>)> {
    let (owner, name) = repo
        .split_once('/')
        .context("Invalid repo format, expected owner/name")?;

    let query = r#"
        query($owner: String!, $name: String!, $pr: Int!, $cursor: String) {
            repository(owner: $owner, name: $name) {
                pullRequest(number: $pr) {
                    number title body state isDraft reviewDecision
                    headRefName baseRefName headRefOid baseRefOid
                    additions deletions changedFiles
                    author { login }
                    reviewRequests(first: 20) {
                        nodes {
                            requestedReviewer {
                                ... on User { login }
                                ... on Team { name }
                            }
                        }
                    }
                    latestReviews(first: 20) {
                        nodes { author { login } state }
                    }
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
    let mut meta: Option<PrMetadata> = None;

    loop {
        let vars = serde_json::json!({
            "owner": owner,
            "name": name,
            "pr": pr_number,
            "cursor": cursor,
        });

        let result = run_graphql(query, &vars).await?;
        let pr = &result["data"]["repository"]["pullRequest"];

        if meta.is_none() {
            let reviewers = parse_reviewers(pr);
            meta = Some(PrMetadata {
                number: pr_number,
                title: pr["title"].as_str().unwrap_or("").to_string(),
                body: pr["body"].as_str().map(String::from),
                state: pr["state"].as_str().unwrap_or("").to_string(),
                draft: pr["isDraft"].as_bool().unwrap_or(false),
                review_decision: pr["reviewDecision"].as_str().map(String::from),
                head: PrRef {
                    sha: pr["headRefOid"].as_str().unwrap_or("").to_string(),
                    ref_name: pr["headRefName"].as_str().unwrap_or("").to_string(),
                },
                base: PrRef {
                    sha: pr["baseRefOid"].as_str().unwrap_or("").to_string(),
                    ref_name: pr["baseRefName"].as_str().unwrap_or("").to_string(),
                },
                user: PrUser {
                    login: pr["author"]["login"].as_str().unwrap_or("").to_string(),
                },
                additions: pr["additions"].as_u64().map(|v| v as usize),
                deletions: pr["deletions"].as_u64().map(|v| v as usize),
                changed_files: pr["changedFiles"].as_u64().map(|v| v as usize),
                reviewers,
            });
        }

        let threads = &pr["reviewThreads"];
        if let Some(nodes) = threads["nodes"].as_array() {
            for node in nodes {
                let thread_id = node["id"].as_str().unwrap_or_default().to_string();
                let is_resolved = node["isResolved"].as_bool().unwrap_or(false);
                if let Some(first_comment) =
                    node["comments"]["nodes"].as_array().and_then(|a| a.first())
                    && let Some(db_id) = first_comment["databaseId"].as_u64()
                {
                    thread_map.insert(
                        db_id,
                        ThreadInfo {
                            thread_node_id: thread_id,
                            is_resolved,
                        },
                    );
                }
            }
        }

        let has_next = threads["pageInfo"]["hasNextPage"]
            .as_bool()
            .unwrap_or(false);
        if has_next {
            cursor = threads["pageInfo"]["endCursor"].as_str().map(String::from);
        } else {
            break;
        }
    }

    let meta = meta.context("GraphQL returned no pullRequest data")?;
    Ok((meta, thread_map))
}

pub async fn update_pr(repo: &str, pr_number: u64, field: &str, value: &str) -> Result<()> {
    let url = format!("repos/{repo}/pulls/{pr_number}");
    let field_arg = format!("{field}={value}");
    run_gh(&["api", &url, "-X", "PATCH", "-f", &field_arg]).await?;
    Ok(())
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
    let mut comments: Vec<ExistingComment> =
        serde_json::from_str(&output).context("Failed to parse review comments")?;

    // Also fetch top-level conversation comments (issues endpoint)
    let issue_url = format!("repos/{repo}/issues/{pr_number}/comments");
    if let Ok(issue_output) = run_gh(&["api", &issue_url, "--paginate"]).await
        && let Ok(issue_comments) = serde_json::from_str::<Vec<IssueComment>>(&issue_output)
    {
        comments.extend(issue_comments.into_iter().map(|ic| ExistingComment {
            id: ic.id,
            path: String::new(),
            line: None,
            side: None,
            start_line: None,
            body: ic.body.unwrap_or_default(),
            user: ic.user,
            created_at: ic.created_at,
            in_reply_to_id: None,
        }));
    }

    Ok(comments)
}

#[derive(Debug, Deserialize)]
struct IssueComment {
    id: u64,
    body: Option<String>,
    user: PrUser,
    created_at: String,
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
    let json_str = serde_json::to_string(&serde_json::json!({ "body": body }))?;
    run_gh_with_stdin(&["api", &url, "-X", "POST", "--input", "-"], &json_str).await?;
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
    run_gh_with_stdin(&["api", &url, "-X", "POST", "--input", "-"], &json_str).await?;
    Ok(())
}

/// Send a JSON body to `gh api` via stdin and return the raw output.
async fn run_gh_with_stdin(args: &[&str], json_body: &str) -> Result<Vec<u8>> {
    use tokio::io::AsyncWriteExt;

    let mut child = Command::new("gh")
        .args(args)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(json_body.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", format_api_error(&stdout, &stderr));
    }

    Ok(output.stdout)
}

async fn run_graphql(query: &str, variables: &serde_json::Value) -> Result<serde_json::Value> {
    let body = serde_json::json!({ "query": query, "variables": variables });
    let json_str = serde_json::to_string(&body)?;
    let stdout = run_gh_with_stdin(&["api", "graphql", "--input", "-"], &json_str).await?;
    serde_json::from_slice(&stdout).context("Failed to parse GraphQL response")
}

/// Batch-fetch full PR data for multiple PRs in a single GraphQL call.
///
/// Returns a map from PR number to parsed data. Used for stack pre-fetching.
/// Note: GraphQL does not provide file patches (diff text), so files only
/// contain metadata. Diff patches are fetched on-demand via REST when the
/// user navigates to a pre-fetched PR.
pub async fn fetch_prs_batch(
    repo: &str,
    pr_numbers: &[u64],
) -> Result<HashMap<u64, (PrMetadata, Vec<ExistingComment>, HashMap<u64, ThreadInfo>)>> {
    if pr_numbers.is_empty() {
        return Ok(HashMap::new());
    }

    let (owner, name) = repo
        .split_once('/')
        .context("Invalid repo format, expected owner/name")?;

    // Build aliased query: pr123: pullRequest(number: 123) { ...PrFields }
    let fragment = r#"
fragment PrFields on PullRequest {
    number title body state isDraft reviewDecision
    headRefName baseRefName headRefOid baseRefOid
    additions deletions changedFiles
    author { login }
    reviewThreads(first: 100) {
        nodes {
            id isResolved
            comments(first: 100) {
                nodes {
                    databaseId body path line startLine createdAt
                    author { login }
                    replyTo { databaseId }
                }
            }
        }
    }
    comments(first: 100) {
        nodes { databaseId body author { login } createdAt }
    }
}
"#;

    let pr_aliases: Vec<String> = pr_numbers
        .iter()
        .map(|n| format!("pr{n}: pullRequest(number: {n}) {{ ...PrFields }}"))
        .collect();

    let query = format!(
        "{fragment}\nquery {{ repository(owner: \"{owner}\", name: \"{name}\") {{ {} }} }}",
        pr_aliases.join("\n")
    );

    let result = run_graphql(&query, &serde_json::json!({})).await?;
    let repo_data = &result["data"]["repository"];

    let mut out = HashMap::new();
    for &pr_number in pr_numbers {
        let key = format!("pr{pr_number}");
        let pr = &repo_data[&key];
        if pr.is_null() {
            continue;
        }

        let meta = PrMetadata {
            number: pr_number,
            title: pr["title"].as_str().unwrap_or("").to_string(),
            body: pr["body"].as_str().map(String::from),
            state: pr["state"].as_str().unwrap_or("").to_string(),
            draft: pr["isDraft"].as_bool().unwrap_or(false),
            review_decision: pr["reviewDecision"].as_str().map(String::from),
            head: PrRef {
                sha: pr["headRefOid"].as_str().unwrap_or("").to_string(),
                ref_name: pr["headRefName"].as_str().unwrap_or("").to_string(),
            },
            base: PrRef {
                sha: pr["baseRefOid"].as_str().unwrap_or("").to_string(),
                ref_name: pr["baseRefName"].as_str().unwrap_or("").to_string(),
            },
            user: PrUser {
                login: pr["author"]["login"].as_str().unwrap_or("").to_string(),
            },
            additions: pr["additions"].as_u64().map(|v| v as usize),
            deletions: pr["deletions"].as_u64().map(|v| v as usize),
            changed_files: pr["changedFiles"].as_u64().map(|v| v as usize),
            reviewers: Vec::new(),
        };

        // Parse review thread comments
        let mut comments = Vec::new();
        let mut threads = HashMap::new();

        if let Some(thread_nodes) = pr["reviewThreads"]["nodes"].as_array() {
            for thread in thread_nodes {
                let thread_id = thread["id"].as_str().unwrap_or("").to_string();
                let is_resolved = thread["isResolved"].as_bool().unwrap_or(false);

                if let Some(comment_nodes) = thread["comments"]["nodes"].as_array() {
                    for (i, c) in comment_nodes.iter().enumerate() {
                        let db_id = c["databaseId"].as_u64().unwrap_or(0);
                        if i == 0 {
                            threads.insert(
                                db_id,
                                ThreadInfo {
                                    thread_node_id: thread_id.clone(),
                                    is_resolved,
                                },
                            );
                        }
                        let reply_to = c["replyTo"]["databaseId"].as_u64();
                        comments.push(ExistingComment {
                            id: db_id,
                            path: c["path"].as_str().unwrap_or("").to_string(),
                            line: c["line"].as_u64().map(|v| v as usize),
                            side: None,
                            start_line: c["startLine"].as_u64().map(|v| v as usize),
                            body: c["body"].as_str().unwrap_or("").to_string(),
                            user: PrUser {
                                login: c["author"]["login"].as_str().unwrap_or("").to_string(),
                            },
                            created_at: c["createdAt"].as_str().unwrap_or("").to_string(),
                            in_reply_to_id: reply_to,
                        });
                    }
                }
            }
        }

        // Parse conversation comments
        if let Some(comment_nodes) = pr["comments"]["nodes"].as_array() {
            for c in comment_nodes {
                comments.push(ExistingComment {
                    id: c["databaseId"].as_u64().unwrap_or(0),
                    path: String::new(),
                    line: None,
                    side: None,
                    start_line: None,
                    body: c["body"].as_str().unwrap_or("").to_string(),
                    user: PrUser {
                        login: c["author"]["login"].as_str().unwrap_or("").to_string(),
                    },
                    created_at: c["createdAt"].as_str().unwrap_or("").to_string(),
                    in_reply_to_id: None,
                });
            }
        }

        out.insert(pr_number, (meta, comments, threads));
    }

    Ok(out)
}

/// Fetch review thread resolve status. Returns a map from root comment database ID
/// to ThreadInfo (thread node_id + is_resolved).
pub async fn fetch_review_threads(repo: &str, pr_number: u64) -> Result<HashMap<u64, ThreadInfo>> {
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
                if let Some(first_comment) =
                    node["comments"]["nodes"].as_array().and_then(|a| a.first())
                    && let Some(db_id) = first_comment["databaseId"].as_u64()
                {
                    thread_map.insert(
                        db_id,
                        ThreadInfo {
                            thread_node_id: thread_id,
                            is_resolved,
                        },
                    );
                }
            }
        }

        let has_next = threads["pageInfo"]["hasNextPage"]
            .as_bool()
            .unwrap_or(false);
        if has_next {
            cursor = threads["pageInfo"]["endCursor"].as_str().map(String::from);
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
        bail!(
            "Line {line_number} is out of range (file has {} lines)",
            lines.len()
        );
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
    run_gh_with_stdin(
        &["api", &update_url, "-X", "PUT", "--input", "-"],
        &json_str,
    )
    .await?;
    Ok(())
}

pub async fn dismiss_review(
    repo: &str,
    pr_number: u64,
    review_id: u64,
    message: &str,
) -> Result<()> {
    let url = format!("repos/{repo}/pulls/{pr_number}/reviews/{review_id}/dismissals");
    let json_str = serde_json::to_string(&serde_json::json!({ "message": message }))?;
    run_gh_with_stdin(&["api", &url, "-X", "PUT", "--input", "-"], &json_str).await?;
    Ok(())
}
