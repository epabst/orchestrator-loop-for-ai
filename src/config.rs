use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SdlcConfig {
    pub states: Vec<StateConfig>,
    pub human_help_label: String,
    pub lock_label: String,
    pub enabled_label: String,
    pub failure_label: String,
    #[serde(default = "default_pr_defined_keyword")]
    pub pr_defined_keyword: String,
    #[serde(default = "default_merge_conflict_keyword")]
    pub merge_conflict_keyword: String,
    #[serde(default)]
    pub auto_rebase: AutoRebaseConfig,
    /// Name of the state new issues start in. Defaults to the first state.
    /// Lets the requirements and design states stay defined but be skipped.
    #[serde(default)]
    pub initial_state: Option<String>,
}

fn default_pr_defined_keyword() -> String {
    "CHANGES_SUMMARIZED".to_string()
}

fn default_merge_conflict_keyword() -> String {
    "MERGE_RESOLVED".to_string()
}

fn default_auto_rebase_enabled() -> bool {
    true
}

fn default_main_branch() -> String {
    "main".to_string()
}

fn default_publish_script() -> String {
    "publish-android-dev.sh".to_string()
}

fn default_rebase_check_interval_secs() -> u64 {
    300
}

fn default_needs_manual_rebase_label() -> String {
    "needs-manual-rebase".to_string()
}

/// Keeps open PRs current with `origin/<main_branch>` and re-publishes their build
/// whenever the base branch moves.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutoRebaseConfig {
    #[serde(default = "default_auto_rebase_enabled")]
    pub enabled: bool,
    #[serde(default = "default_main_branch")]
    pub main_branch: String,
    /// Script (relative to the repo root) run after a successful rebase. Skipped when absent.
    #[serde(default = "default_publish_script")]
    pub publish_script: String,
    /// How often to fetch and compare `origin/<main_branch>`.
    #[serde(default = "default_rebase_check_interval_secs")]
    pub check_interval_secs: u64,
    /// Applied to the issue when an auto-rebase hits conflicts a resolver agent
    /// can't clear, instead of posting a fresh comment on every `main` move.
    /// Removed again once the branch rebases cleanly or its PR is merged.
    #[serde(default = "default_needs_manual_rebase_label")]
    pub needs_manual_rebase_label: String,
}

impl Default for AutoRebaseConfig {
    fn default() -> Self {
        Self {
            enabled: default_auto_rebase_enabled(),
            main_branch: default_main_branch(),
            publish_script: default_publish_script(),
            check_interval_secs: default_rebase_check_interval_secs(),
            needs_manual_rebase_label: default_needs_manual_rebase_label(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateConfig {
    pub name: String,
    pub label: String,
    /// Instructions for the agent. Empty for a natively handled state, which
    /// runs orchestrator code instead of invoking an agent.
    #[serde(alias = "prompt_suffix", default)]
    pub prompt: String,
    pub next_state: Option<String>,
    #[serde(default)]
    pub keywords: HashMap<String, String>,
    #[serde(default)]
    pub doc_file: Option<String>,
    /// Names orchestrator code that advances this state instead of an agent.
    /// The only handler today is `await-pr-merge`.
    #[serde(default)]
    pub native_handler: Option<String>,
}

/// `native_handler` value for the state that waits for the PR to be merged,
/// publishes a build of main and then moves the issue on to its next state.
pub const AWAIT_PR_MERGE_HANDLER: &str = "await-pr-merge";

impl SdlcConfig {
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    #[allow(dead_code)]
    pub fn get_state(&self, label: &str) -> Option<&StateConfig> {
        self.states.iter().find(|s| s.label == label)
    }

    /// The state a new issue starts in: `initial_state` when it names a
    /// configured state, otherwise the first state.
    pub fn initial_state(&self) -> &StateConfig {
        self.initial_state
            .as_deref()
            .and_then(|n| self.states.iter().find(|s| s.name == n))
            .unwrap_or(&self.states[0])
    }

    /// The state whose merge the orchestrator watches for, when one is configured.
    pub fn await_pr_merge_state(&self) -> Option<&StateConfig> {
        self.states
            .iter()
            .find(|s| s.native_handler.as_deref() == Some(AWAIT_PR_MERGE_HANDLER))
    }

    /// Label of that state, so callers can test an issue for it without
    /// caring whether the state is configured at all.
    pub fn await_pr_merge_label(&self) -> Option<&str> {
        self.await_pr_merge_state().map(|s| s.label.as_str())
    }

    /// Label an issue moves to once the merge it was waiting for is published:
    /// the label of the state's `next_state`, falling back to `ai-done`.
    pub fn label_after_pr_merge_wait(&self) -> String {
        let next = self
            .await_pr_merge_state()
            .and_then(|s| s.next_state.clone())
            .unwrap_or_else(|| "ai-done".to_string());
        self.states
            .iter()
            .find(|s| s.name == next)
            .map(|s| s.label.clone())
            .unwrap_or(next)
    }
}

impl StateConfig {
    pub fn get_labels_for_keyword(&self, keyword: &str) -> Vec<String> {
        self.keywords
            .get(keyword)
            .map(|label_str| {
                label_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
