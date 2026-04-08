use crate::types::{DiffFile, LineKind, PrMetadata};

const MAX_DIFF_CHARS: usize = 100_000;

pub const DESCRIPTION_SYSTEM_PROMPT: &str = "\
You generate pull request titles and descriptions from code diffs.

Output format (strictly follow this):
- Line 1: the PR title (concise, imperative mood, no prefix)
- Line 2: blank
- Lines 3+: the PR body in GitHub-flavored markdown

The body should:
- Start with a one-sentence summary of what changed and why
- Use bullet points for individual changes
- Mention any important design decisions or trade-offs
- Be concise but informative

Do not include anything else in your response. No preamble, no commentary.";

/// Build the user prompt from PR metadata, current title/description, and diff files.
pub fn build_description_prompt(
    files: &[DiffFile],
    meta: Option<&PrMetadata>,
    current_title: &str,
    current_body: &str,
) -> String {
    let mut out = String::with_capacity(MAX_DIFF_CHARS + 2048);

    if let Some(m) = meta {
        out.push_str(&format!("PR #{}\n", m.number));
        out.push_str(&format!("Branch: {} → {}\n", m.base.ref_name, m.head.ref_name));
        out.push_str(&format!("Author: {}\n", m.user.login));
    }

    if !current_title.is_empty() {
        out.push_str(&format!("Current title: {}\n", current_title));
    }
    if !current_body.trim().is_empty() {
        out.push_str(&format!("Current description:\n{}\n", current_body.trim()));
    }
    out.push('\n');

    out.push_str("Diff:\n\n");

    let mut remaining = MAX_DIFF_CHARS;
    let mut truncated_files = 0usize;

    for file in files {
        let file_diff = format_file_diff(file);
        if file_diff.len() > remaining {
            truncated_files += files.len() - files.iter().position(|f| std::ptr::eq(f, file)).unwrap_or(0);
            break;
        }
        remaining -= file_diff.len();
        out.push_str(&file_diff);
    }

    if truncated_files > 0 {
        out.push_str(&format!("\n... ({truncated_files} more file(s) truncated)\n"));
    }

    out
}

/// Parse the Claude response into (title, body).
pub fn parse_title_body(response: &str) -> (String, String) {
    let trimmed = response.trim();
    if let Some((title, rest)) = trimmed.split_once('\n') {
        let body = rest.trim_start_matches('\n').to_string();
        (title.trim().to_string(), body)
    } else {
        (trimmed.to_string(), String::new())
    }
}

fn format_file_diff(file: &DiffFile) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "--- a/{}\n+++ b/{}\n",
        file.path, file.path
    ));

    for hunk in &file.hunks {
        out.push_str(&hunk.header);
        if !hunk.header.ends_with('\n') {
            out.push('\n');
        }
        for line in &hunk.lines {
            let prefix = match line.kind {
                LineKind::Context => ' ',
                LineKind::Added => '+',
                LineKind::Removed => '-',
            };
            out.push(prefix);
            out.push_str(&line.content);
            if !line.content.ends_with('\n') {
                out.push('\n');
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_title_and_body() {
        let response = "Add user authentication\n\nImplement JWT-based auth.\n\n- Added login endpoint\n- Added middleware";
        let (title, body) = parse_title_body(response);
        assert_eq!(title, "Add user authentication");
        assert!(body.starts_with("Implement JWT-based auth."));
    }

    #[test]
    fn parse_title_only() {
        let (title, body) = parse_title_body("Fix typo in README");
        assert_eq!(title, "Fix typo in README");
        assert!(body.is_empty());
    }

    #[test]
    fn format_diff_roundtrip() {
        use crate::types::{DiffLine, FileStatus, Hunk};

        let file = DiffFile {
            path: "src/main.rs".to_string(),
            status: FileStatus::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![Hunk {
                header: "@@ -1,3 +1,4 @@".to_string(),
                old_start: 1,
                old_count: 3,
                new_start: 1,
                new_count: 4,
                lines: vec![
                    DiffLine {
                        kind: LineKind::Context,
                        old_lineno: Some(1),
                        new_lineno: Some(1),
                        content: "fn main() {".to_string(),
                        highlighted_content: None,
                    },
                    DiffLine {
                        kind: LineKind::Added,
                        old_lineno: None,
                        new_lineno: Some(2),
                        content: "    println!(\"hello\");".to_string(),
                        highlighted_content: None,
                    },
                ],
            }],
        };

        let output = format_file_diff(&file);
        assert!(output.contains("--- a/src/main.rs"));
        assert!(output.contains("+++ b/src/main.rs"));
        assert!(output.contains("+    println!(\"hello\");"));
    }
}
