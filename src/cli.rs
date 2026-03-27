use anyhow::{Context, Result, bail};
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "gh-review",
    version,
    about = "Terminal UI for reviewing GitHub pull requests",
    after_help = "\x1b[1mExamples:\x1b[0m
  gh-review 42                                             From inside a git repo
  gh-review octocat/hello-world 42                         Explicit owner/repo
  gh-review https://github.com/owner/repo/pull/42          GitHub PR URL
  gh-review https://app.graphite.com/github/pr/org/repo/1  Graphite URL"
)]
pub struct Cli {
    /// PR number, owner/repo + number, or a PR URL
    #[arg(value_name = "ARGS")]
    pub args: Vec<String>,
}

pub fn resolve(args: Vec<String>) -> Result<(String, u64)> {
    match args.len() {
        1 => {
            let arg = &args[0];
            if let Some((repo, pr)) = parse_github_url(arg) {
                return Ok((repo, pr));
            }
            if let Some((repo, pr)) = parse_graphite_url(arg) {
                return Ok((repo, pr));
            }
            let pr: u64 = arg
                .parse()
                .context("Single argument must be a PR number or URL")?;
            let repo = repo_from_git_remote().context(
                "Could not infer repo from git remote. Run from inside a git repo or pass owner/repo explicitly.",
            )?;
            Ok((repo, pr))
        }
        2 => {
            let repo = args[0].clone();
            let pr: u64 = args[1]
                .parse()
                .context("Second argument must be a PR number")?;
            Ok((repo, pr))
        }
        0 => bail!(
            "No arguments provided.\n\n\
             Usage:\n  \
             gh-review 42                              From inside a git repo\n  \
             gh-review owner/repo 42                   Explicit owner/repo\n  \
             gh-review <PR_URL>                        GitHub or Graphite URL\n\n\
             Run gh-review --help for more info."
        ),
        _ => bail!("Too many arguments. Usage: gh-review [OWNER/REPO] <PR_NUMBER>"),
    }
}

fn strip_query_params(s: &str) -> &str {
    s.split('?').next().unwrap_or(s)
}

fn parse_github_url(s: &str) -> Option<(String, u64)> {
    let rest = strip_query_params(s).strip_prefix("https://github.com/")?;
    let parts: Vec<&str> = rest.splitn(4, '/').collect();
    if parts.len() >= 4 && parts[2] == "pull" {
        let pr: u64 = parts[3].trim_end_matches('/').parse().ok()?;
        Some((format!("{}/{}", parts[0], parts[1]), pr))
    } else {
        None
    }
}

fn parse_graphite_url(s: &str) -> Option<(String, u64)> {
    let rest = strip_query_params(s).strip_prefix("https://app.graphite.com/github/pr/")?;
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.len() >= 3 {
        let pr: u64 = parts[2].trim_end_matches('/').parse().ok()?;
        Some((format!("{}/{}", parts[0], parts[1]), pr))
    } else {
        None
    }
}

fn repo_from_git_remote() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8(output.stdout).ok()?.trim().to_string();
    parse_repo_from_remote_url(&url)
}

fn parse_repo_from_remote_url(url: &str) -> Option<String> {
    let raw = if let Some(rest) = url.strip_prefix("git@github.com:") {
        rest.to_string()
    } else if let Some(rest) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
    {
        rest.to_string()
    } else if let Some(rest) = url.strip_prefix("ssh://git@github.com/") {
        rest.to_string()
    } else {
        return None;
    };
    let repo = raw.strip_suffix(".git").unwrap_or(&raw);
    let repo = repo.trim_end_matches('/');
    if repo.contains('/') {
        Some(repo.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- GitHub URL parsing ---

    #[test]
    fn parse_github_url_standard() {
        let (repo, pr) =
            parse_github_url("https://github.com/Neville-Loh/gh-review/pull/32").unwrap();
        assert_eq!(repo, "Neville-Loh/gh-review");
        assert_eq!(pr, 32);
    }

    #[test]
    fn parse_github_url_trailing_slash() {
        let (repo, pr) =
            parse_github_url("https://github.com/octocat/hello-world/pull/42/").unwrap();
        assert_eq!(repo, "octocat/hello-world");
        assert_eq!(pr, 42);
    }

    #[test]
    fn parse_github_url_with_query_params() {
        let (repo, pr) = parse_github_url(
            "https://github.com/owner/repo/pull/99?diff=split&w=1",
        )
        .unwrap();
        assert_eq!(repo, "owner/repo");
        assert_eq!(pr, 99);
    }

    #[test]
    fn parse_github_url_not_a_pr() {
        assert!(parse_github_url("https://github.com/octocat/hello-world/issues/5").is_none());
    }

    #[test]
    fn parse_github_url_not_github() {
        assert!(parse_github_url("https://gitlab.com/foo/bar/pull/1").is_none());
    }

    // --- Graphite URL parsing ---

    #[test]
    fn parse_graphite_url_standard() {
        let (repo, pr) =
            parse_graphite_url("https://app.graphite.com/github/pr/ROKT/srs/12569").unwrap();
        assert_eq!(repo, "ROKT/srs");
        assert_eq!(pr, 12569);
    }

    #[test]
    fn parse_graphite_url_trailing_slash() {
        let (repo, pr) =
            parse_graphite_url("https://app.graphite.com/github/pr/owner/repo/99/").unwrap();
        assert_eq!(repo, "owner/repo");
        assert_eq!(pr, 99);
    }

    #[test]
    fn parse_graphite_url_with_query_params() {
        let (repo, pr) = parse_graphite_url(
            "https://app.graphite.com/github/pr/ROKT/predictor/9780?ref=gt-pasteable-stack&onboarding_state=not-authorized",
        )
        .unwrap();
        assert_eq!(repo, "ROKT/predictor");
        assert_eq!(pr, 9780);
    }

    #[test]
    fn parse_graphite_url_not_graphite() {
        assert!(parse_graphite_url("https://example.com/github/pr/foo/bar/1").is_none());
    }

    // --- Git remote URL parsing ---

    #[test]
    fn remote_ssh() {
        let repo = parse_repo_from_remote_url("git@github.com:Neville-Loh/gh-review.git");
        assert_eq!(repo.unwrap(), "Neville-Loh/gh-review");
    }

    #[test]
    fn remote_https_with_git_suffix() {
        let repo = parse_repo_from_remote_url("https://github.com/Neville-Loh/gh-review.git");
        assert_eq!(repo.unwrap(), "Neville-Loh/gh-review");
    }

    #[test]
    fn remote_https_no_git_suffix() {
        let repo = parse_repo_from_remote_url("https://github.com/octocat/hello-world");
        assert_eq!(repo.unwrap(), "octocat/hello-world");
    }

    #[test]
    fn remote_https_trailing_slash() {
        let repo = parse_repo_from_remote_url("https://github.com/octocat/hello-world/");
        assert_eq!(repo.unwrap(), "octocat/hello-world");
    }

    #[test]
    fn remote_ssh_protocol() {
        let repo = parse_repo_from_remote_url("ssh://git@github.com/owner/repo.git");
        assert_eq!(repo.unwrap(), "owner/repo");
    }

    #[test]
    fn remote_http() {
        let repo = parse_repo_from_remote_url("http://github.com/owner/repo.git");
        assert_eq!(repo.unwrap(), "owner/repo");
    }

    #[test]
    fn remote_non_github() {
        assert!(parse_repo_from_remote_url("git@gitlab.com:foo/bar.git").is_none());
    }

    // --- resolve_args ---

    #[test]
    fn resolve_two_args() {
        let (repo, pr) = resolve(vec!["owner/repo".into(), "42".into()]).unwrap();
        assert_eq!(repo, "owner/repo");
        assert_eq!(pr, 42);
    }

    #[test]
    fn resolve_github_url() {
        let (repo, pr) = resolve(vec![
            "https://github.com/Neville-Loh/gh-review/pull/32".into(),
        ])
        .unwrap();
        assert_eq!(repo, "Neville-Loh/gh-review");
        assert_eq!(pr, 32);
    }

    #[test]
    fn resolve_graphite_url() {
        let (repo, pr) = resolve(vec![
            "https://app.graphite.com/github/pr/ROKT/srs/12569".into(),
        ])
        .unwrap();
        assert_eq!(repo, "ROKT/srs");
        assert_eq!(pr, 12569);
    }

    #[test]
    fn resolve_zero_args_is_error() {
        assert!(resolve(vec![]).is_err());
    }

    #[test]
    fn resolve_three_args_is_error() {
        assert!(resolve(vec!["a".into(), "b".into(), "c".into()]).is_err());
    }

    #[test]
    fn resolve_non_numeric_single_arg_is_error() {
        assert!(resolve(vec!["not-a-number".into()]).is_err());
    }

    #[test]
    fn resolve_non_numeric_second_arg_is_error() {
        assert!(resolve(vec!["owner/repo".into(), "abc".into()]).is_err());
    }
}
