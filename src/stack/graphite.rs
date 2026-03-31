//! Graphite stack comment detection and PR link extraction.
//!
//! Graphite posts a comment on each PR in a stack with merge queue
//! instructions and a list of linked PRs. These comments are filtered
//! from the diff view and their links are extracted for the stack display.

use crate::types::ExistingComment;

use super::PrLink;

const GRAPHITE_MARKER: &str = "managed by";
const GRAPHITE_DOMAIN: &str = "graphite.dev";
const GRAPHITE_PR_PREFIX: &str = "https://app.graphite.com/github/pr/";

/// Returns true if the comment body is an auto-generated Graphite stack comment.
pub fn is_graphite_stack_comment(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains(GRAPHITE_MARKER) && lower.contains(GRAPHITE_DOMAIN)
}

/// Scan comments for a Graphite stack comment and return the extracted PR links.
pub fn extract_stack(comments: &[ExistingComment]) -> Vec<PrLink> {
    comments
        .iter()
        .find(|c| is_graphite_stack_comment(&c.body))
        .map(|c| extract_pr_links(&c.body))
        .unwrap_or_default()
}

/// Extract all Graphite PR links from a comment body.
///
/// Links have the form:
/// `https://app.graphite.com/github/pr/{OWNER}/{REPO}/{PR_NUMBER}?...`
fn extract_pr_links(body: &str) -> Vec<PrLink> {
    let mut links = Vec::new();
    let mut search_from = 0;

    while let Some(start) = body[search_from..].find(GRAPHITE_PR_PREFIX) {
        let abs_start = search_from + start;
        let path_start = abs_start + GRAPHITE_PR_PREFIX.len();

        let path_end = body[path_start..]
            .find(['"', '\'', ')', ' ', '>', '\n'])
            .map(|i| path_start + i)
            .unwrap_or(body.len());

        let full_url = &body[abs_start..path_end];
        let path = &body[path_start..path_end];

        let clean_path = path.split('?').next().unwrap_or(path);
        let segments: Vec<&str> = clean_path.split('/').collect();

        if segments.len() >= 3
            && let Ok(pr_number) = segments[2].parse::<u64>()
        {
            links.push(PrLink {
                owner: segments[0].to_string(),
                repo: segments[1].to_string(),
                pr_number,
                url: full_url.to_string(),
            });
        }

        search_from = path_end;
    }

    links.sort_by_key(|l| l.pr_number);
    links.dedup_by_key(|l| l.pr_number);
    links
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_GRAPHITE_COMMENT: &str = r#"
> [!WARNING]
> <b>This pull request is not mergeable via GitHub because a downstack PR is open.</b>

* **#104** <a href="https://app.graphite.com/github/pr/acme/widgets/104?utm_source=stack-comment-icon" target="_blank">View</a> 👈
* **#103** <a href="https://app.graphite.com/github/pr/acme/widgets/103?utm_source=stack-comment-icon" target="_blank">View</a>
* **#102** <a href="https://app.graphite.com/github/pr/acme/widgets/102?utm_source=stack-comment-icon" target="_blank">View</a>
* **#101** <a href="https://app.graphite.com/github/pr/acme/widgets/101?utm_source=stack-comment-icon" target="_blank">View</a>
* `main`

This stack of pull requests is managed by <a href="https://graphite.dev?utm-source=stack-comment"><b>Graphite</b></a>. Learn more about <a href="https://stacking.dev/?utm_source=stack-comment">stacking</a>.
"#;

    #[test]
    fn detects_real_graphite_comment() {
        assert!(is_graphite_stack_comment(REAL_GRAPHITE_COMMENT));
    }

    #[test]
    fn rejects_normal_comment() {
        assert!(!is_graphite_stack_comment("This looks good, LGTM!"));
    }

    #[test]
    fn rejects_comment_mentioning_graphite_without_marker() {
        assert!(!is_graphite_stack_comment(
            "I used graphite to create this PR"
        ));
    }

    #[test]
    fn rejects_empty_body() {
        assert!(!is_graphite_stack_comment(""));
    }

    #[test]
    fn rejects_partial_marker() {
        assert!(!is_graphite_stack_comment("managed by someone else"));
    }

    #[test]
    fn extracts_pr_links_from_real_comment() {
        let links = extract_pr_links(REAL_GRAPHITE_COMMENT);
        assert_eq!(links.len(), 4);
        assert_eq!(links[0].pr_number, 101);
        assert_eq!(links[0].owner, "acme");
        assert_eq!(links[0].repo, "widgets");
        assert_eq!(links[1].pr_number, 102);
        assert_eq!(links[2].pr_number, 103);
        assert_eq!(links[3].pr_number, 104);
    }

    #[test]
    fn extracts_links_with_query_params() {
        let body =
            r#"<a href="https://app.graphite.com/github/pr/org/repo/42?utm_source=test">link</a>"#;
        let links = extract_pr_links(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].pr_number, 42);
        assert_eq!(links[0].owner, "org");
        assert_eq!(links[0].repo, "repo");
    }

    #[test]
    fn returns_empty_for_no_links() {
        assert!(extract_pr_links("Just a normal comment").is_empty());
    }

    #[test]
    fn deduplicates_repeated_links() {
        let body = r#"
<a href="https://app.graphite.com/github/pr/org/repo/100?a=1">first</a>
<a href="https://app.graphite.com/github/pr/org/repo/100?a=2">second</a>
"#;
        let links = extract_pr_links(body);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].pr_number, 100);
    }

    #[test]
    fn extract_stack_from_comments() {
        let comments = vec![
            ExistingComment {
                id: 1,
                path: String::new(),
                line: None,
                side: None,
                start_line: None,
                body: "normal comment".to_string(),
                user: crate::types::PrUser {
                    login: "alice".to_string(),
                },
                created_at: String::new(),
                in_reply_to_id: None,
            },
            ExistingComment {
                id: 2,
                path: String::new(),
                line: None,
                side: None,
                start_line: None,
                body: REAL_GRAPHITE_COMMENT.to_string(),
                user: crate::types::PrUser {
                    login: "graphite-app".to_string(),
                },
                created_at: String::new(),
                in_reply_to_id: None,
            },
        ];
        let links = extract_stack(&comments);
        assert_eq!(links.len(), 4);
    }
}
