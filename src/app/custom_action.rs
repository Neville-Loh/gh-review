use std::collections::HashMap;

use crate::config::{KeyBinding, RawAction, parse_key_string};
use crate::event::AppEvent;

use super::App;
use super::keymap::KeyCombo;

/// A user-defined command that runs a shell process.
/// Can be triggered by a hotkey, by typing `:name` in the command bar, or both.
#[derive(Clone, Debug)]
pub struct CustomAction {
    pub name: String,
    pub command_template: String,
    pub description: String,
}

/// Variables available for template expansion in custom action commands.
pub struct ActionVars {
    pub pr_number: u64,
    pub repo: String,
    pub repo_owner: String,
    pub repo_name: String,
    pub url: String,
    pub branch: String,
    pub base_branch: String,
}

impl CustomAction {
    pub fn expand(&self, vars: &ActionVars) -> String {
        self.command_template
            .replace("{PR_NUMBER}", &vars.pr_number.to_string())
            .replace("{REPO}", &vars.repo)
            .replace("{REPO_OWNER}", &vars.repo_owner)
            .replace("{REPO_NAME}", &vars.repo_name)
            .replace("{URL}", &vars.url)
            .replace("{BRANCH}", &vars.branch)
            .replace("{BASE_BRANCH}", &vars.base_branch)
    }
}

/// Resolved custom actions: keyed actions (for hotkey dispatch) and all actions (for command bar).
pub struct ResolvedActions {
    pub keyed: HashMap<KeyCombo, CustomAction>,
    pub all: Vec<CustomAction>,
    pub warnings: Vec<String>,
}

/// Parse raw actions from config into resolved keyed + named actions.
pub fn resolve_custom_actions(raw: &[RawAction]) -> ResolvedActions {
    let mut keyed = HashMap::new();
    let mut all = Vec::new();
    let mut warnings = Vec::new();

    for action in raw {
        let description = if action.description.is_empty() {
            action.command.clone()
        } else {
            action.description.clone()
        };

        let custom = CustomAction {
            name: action.name.clone(),
            command_template: action.command.clone(),
            description,
        };

        if !action.key.is_empty() && action.key != "no_op" {
            match parse_key_string(&action.key) {
                Some(KeyBinding::Single(combo)) => {
                    keyed.insert(combo, custom.clone());
                }
                Some(KeyBinding::Pending { .. }) => {
                    warnings.push(format!(
                        "Pending sequences not supported for custom actions: {:?}",
                        action.key
                    ));
                }
                None => {
                    warnings.push(format!("Invalid key for custom action: {:?}", action.key));
                }
            }
        }

        all.push(custom);
    }

    ResolvedActions {
        keyed,
        all,
        warnings,
    }
}

impl App {
    pub(crate) fn action_vars(&self) -> ActionVars {
        let (repo_owner, repo_name) = self.repo.split_once('/').unwrap_or(("", &self.repo));
        let branch = self
            .pr_meta
            .as_ref()
            .map(|m| m.head.ref_name.clone())
            .unwrap_or_default();
        let base_branch = self
            .pr_meta
            .as_ref()
            .map(|m| m.base.ref_name.clone())
            .unwrap_or_default();
        ActionVars {
            pr_number: self.pr_number,
            repo: self.repo.clone(),
            repo_owner: repo_owner.to_string(),
            repo_name: repo_name.to_string(),
            url: format!("https://github.com/{}/pull/{}", self.repo, self.pr_number),
            branch,
            base_branch,
        }
    }

    pub(crate) fn run_custom_action(&mut self, action: &CustomAction) {
        let vars = self.action_vars();
        let command = action.expand(&vars);
        let description = action.description.clone();

        self.status.info(format!("Running: {description}..."));

        let tx = self.tx.clone();
        tokio::spawn(async move {
            let output = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output()
                .await;

            match output {
                Ok(out) if out.status.success() => {
                    let _ = tx.send(AppEvent::CustomActionComplete {
                        description,
                        result: Ok(()),
                    });
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    let msg = if stderr.is_empty() {
                        format!("Command failed with {}", out.status)
                    } else {
                        stderr
                    };
                    let _ = tx.send(AppEvent::CustomActionComplete {
                        description,
                        result: Err(msg),
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::CustomActionComplete {
                        description,
                        result: Err(format!("Failed to run: {e}")),
                    });
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vars() -> ActionVars {
        ActionVars {
            pr_number: 42,
            repo: "octocat/hello-world".to_string(),
            repo_owner: "octocat".to_string(),
            repo_name: "hello-world".to_string(),
            url: "https://github.com/octocat/hello-world/pull/42".to_string(),
            branch: "feature/my-branch".to_string(),
            base_branch: "main".to_string(),
        }
    }

    #[test]
    fn expand_all_variables() {
        let action = CustomAction {
            name: "lgtm".to_string(),
            command_template: "gh pr comment {PR_NUMBER} --repo {REPO} --body 'LGTM'".to_string(),
            description: "Comment LGTM".to_string(),
        };
        assert_eq!(
            action.expand(&test_vars()),
            "gh pr comment 42 --repo octocat/hello-world --body 'LGTM'"
        );
    }

    #[test]
    fn expand_url_and_branch() {
        let action = CustomAction {
            name: String::new(),
            command_template: "echo {URL} {BRANCH} {BASE_BRANCH}".to_string(),
            description: "Test".to_string(),
        };
        assert_eq!(
            action.expand(&test_vars()),
            "echo https://github.com/octocat/hello-world/pull/42 feature/my-branch main"
        );
    }

    #[test]
    fn expand_owner_and_name() {
        let action = CustomAction {
            name: String::new(),
            command_template: "gh api repos/{REPO_OWNER}/{REPO_NAME}/pulls/{PR_NUMBER}".to_string(),
            description: "API call".to_string(),
        };
        assert_eq!(
            action.expand(&test_vars()),
            "gh api repos/octocat/hello-world/pulls/42"
        );
    }

    #[test]
    fn no_variables_unchanged() {
        let action = CustomAction {
            name: String::new(),
            command_template: "echo hello world".to_string(),
            description: "Static".to_string(),
        };
        assert_eq!(action.expand(&test_vars()), "echo hello world");
    }
}
