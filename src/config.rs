use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SdlcConfig {
    pub states: Vec<StateConfig>,
    pub human_help_label: String,
    pub lock_label: String,
    pub enabled_label: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateConfig {
    pub name: String,
    pub label: String,
    pub prompt_suffix: String,
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

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
