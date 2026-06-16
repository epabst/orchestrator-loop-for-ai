use std::path::{PathBuf};
use std::fs;
use std::collections::HashMap;
use fs2::FileExt;
use anyhow::{Result, anyhow};

pub struct Workspace {
    pub base_dir: PathBuf,
}

impl Workspace {
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
        let base_dir = home.join("ai-workspaces");
        if !base_dir.exists() {
            fs::create_dir_all(&base_dir)?;
        }
        Ok(Self { base_dir })
    }

    pub fn get_issue_dir(&self, repo_name: &str, issue_id: u64) -> PathBuf {
        self.base_dir.join(format!("{}-{}", repo_name, issue_id))
    }

    pub fn create_issue_dir(&self, repo_name: &str, issue_id: u64) -> Result<PathBuf> {
        let dir = self.get_issue_dir(repo_name, issue_id);
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(dir)
    }

    pub fn acquire_lock(&self, repo_name: &str, issue_id: u64) -> Result<fs::File> {
        let dir = self.get_issue_dir(repo_name, issue_id);
        let lock_file_path = dir.join(".lock");
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(lock_file_path)?;

        // Try to lock the file. If it fails, another process has it.
        file.try_lock_exclusive()
            .map_err(|_| anyhow!("Could not acquire lock for issue {}", issue_id))?;
        
        Ok(file)
    }

    pub fn read_config(&self) -> Result<AgentConfig> {
        let config_path = self.base_dir.join("config.yaml");
        if !config_path.exists() {
            // Create default config
            // Default agent is claude. The {prompt} token is replaced with the full prompt
            // as a single CLI argument. For stdin-based agents (e.g. gemini), omit {prompt}
            // and set command_template: "gemini"
            let default_config = AgentConfig {
                command_template: "claude --print {prompt}".to_string(),
                agents: vec![("default".to_string(), "claude".to_string())]
                    .into_iter()
                    .collect(),
                state_agents: HashMap::new(),
            };
            let yaml = serde_yaml::to_string(&default_config)?;
            fs::write(&config_path, yaml)?;
            return Ok(default_config);
        }
        let content = fs::read_to_string(config_path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn get_audit_dir(&self, repo_name: &str, issue_id: u64) -> Result<PathBuf> {
        let audit_dir = self.base_dir.join("audit").join(repo_name).join(issue_id.to_string());
        if !audit_dir.exists() {
            fs::create_dir_all(&audit_dir)?;
        }
        Ok(audit_dir)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentConfig {
    pub command_template: String,
    pub agents: std::collections::HashMap<String, String>,
    pub state_agents: std::collections::HashMap<String, String>,
}

impl AgentConfig {
    pub fn get_command_for_state(&self, state: &str) -> String {
        let agent = self.state_agents
            .get(state)
            .cloned()
            .unwrap_or_else(|| self.agents.get("default").cloned().unwrap_or_else(|| "gemini".to_string()));
        
        self.command_template.replace("{agent}", &agent)
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;
