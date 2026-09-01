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
}

impl Default for AutoRebaseConfig {
    fn default() -> Self {
        Self {
            enabled: default_auto_rebase_enabled(),
            main_branch: default_main_branch(),
            publish_script: default_publish_script(),
            check_interval_secs: default_rebase_check_interval_secs(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateConfig {
    pub name: String,
    pub label: String,
    #[serde(alias = "prompt_suffix")]
    pub prompt: String,
    pub next_state: Option<String>,
    pub keywords: HashMap<String, String>,
    pub doc_file: Option<String>,
}

impl SdlcConfig {
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    #[allow(dead_code)]
    pub fn get_state(&self, label: &str) -> Option<&StateConfig> {
        self.states.iter().find(|s| s.label == label)
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
