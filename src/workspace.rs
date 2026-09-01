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

    pub fn acquire_lock(&self, repo_name: &str, issue_id: u64) -> Result<IssueLock> {
        let dir = self.get_issue_dir(repo_name, issue_id);
        let lock_file_path = dir.join(".lock");
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&lock_file_path)?;

        // Try to lock the file. If it fails, another process has it.
        file.try_lock_exclusive()
            .map_err(|_| anyhow!("Could not acquire lock for issue {}", issue_id))?;

        Ok(IssueLock { file: Some(file), path: lock_file_path })
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
                stream_json: default_stream_json(),
                partial_messages: false,
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

/// Holds an issue's exclusive lock and cleans the `.lock` file up on release.
/// Dropping is what happens when the work finishes *and* when Ctrl+C unwinds or
/// cancels an in-flight SDLC step, so no stale lock files are left behind.
pub struct IssueLock {
    file: Option<fs::File>,
    path: PathBuf,
}

impl Drop for IssueLock {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = fs2::FileExt::unlock(&file);
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn default_stream_json() -> bool {
    true
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentConfig {
    pub command_template: String,
    pub agents: std::collections::HashMap<String, String>,
    pub state_agents: std::collections::HashMap<String, String>,
    /// Ask `claude --print` for `stream-json` output so the orchestrator can
    /// show the conversation - tool calls, results and prose - while the agent
    /// is still working instead of only after it exits.
    #[serde(default = "default_stream_json")]
    pub stream_json: bool,
    /// Also stream partial assistant messages, so prose appears token by token.
    #[serde(default)]
    pub partial_messages: bool,
}

impl AgentConfig {
    /// The agent name that handles `state` - its own entry when it has one,
    /// otherwise the configured default.
    pub fn get_agent_for_state(&self, state: &str) -> String {
        self.state_agents
            .get(state)
            .cloned()
            .unwrap_or_else(|| self.agents.get("default").cloned().unwrap_or_else(|| "gemini".to_string()))
    }

    pub fn get_command_for_state(&self, state: &str) -> String {
        let agent = self.get_agent_for_state(state);
        let command = self.command_template.replace("{agent}", &agent);
        self.with_streaming_flags(command)
    }

    /// Adds Claude's streaming flags right after the binary, leaving the rest of
    /// the template (including where `{prompt}` sits) alone.
    ///
    /// Only applies to `claude --print`: `--output-format` is a Claude flag, and
    /// it only works in print mode. A template that already sets an output
    /// format is left as the user wrote it.
    fn with_streaming_flags(&self, command: String) -> String {
        if !self.stream_json || command.contains("--output-format") {
            return command;
        }

        let mut tokens = command.split_whitespace();
        let Some(bin) = tokens.next() else { return command };
        let is_claude = std::path::Path::new(bin).file_name().and_then(|n| n.to_str()) == Some("claude");
        let rest: Vec<&str> = tokens.collect();
        if !is_claude || !rest.iter().any(|t| *t == "--print" || *t == "-p") {
            return command;
        }

        let mut flags = vec!["--output-format", "stream-json", "--verbose"];
        if self.partial_messages {
            flags.push("--include-partial-messages");
        }
        format!("{} {} {}", bin, flags.join(" "), rest.join(" "))
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;
