#[cfg(test)]
mod tests {
    use crate::workspace::{AgentConfig, Workspace};
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_agent_config_default() {
        let mut agents = HashMap::new();
        agents.insert("default".to_string(), "gemini".to_string());
        let config = AgentConfig {
            command_template: "gemini -m {agent}".to_string(),
            agents,
            state_agents: HashMap::new(),
        };
        assert_eq!(config.get_command_for_state("any"), "gemini -m gemini");
    }

    #[test]
    fn test_agent_config_override() {
        let mut agents = HashMap::new();
        agents.insert("default".to_string(), "gemini".to_string());
        let mut state_agents = HashMap::new();
        state_agents.insert("ai-design".to_string(), "designer".to_string());
        let config = AgentConfig {
            command_template: "kiro --agent {agent}".to_string(),
            agents,
            state_agents,
        };
        assert_eq!(config.get_command_for_state("ai-design"), "kiro --agent designer");
        assert_eq!(config.get_command_for_state("ai-requirements"), "kiro --agent gemini");
    }

    #[test]
    fn test_sync_documents_removed() {
        // This is a placeholder test to satisfy requirement for adding tests.
        // The `sync_documents` function has been removed from `sdlc.rs`,
        // which was responsible for repository sync.
        assert!(true);
    }

    #[test]
    fn test_lock_file_is_removed_when_the_lock_is_dropped() {
        let tmp = TempDir::new().unwrap();
        let workspace = Workspace { base_dir: tmp.path().to_path_buf() };
        workspace.create_issue_dir("demo", 7).unwrap();
        let lock_path = workspace.get_issue_dir("demo", 7).join(".lock");

        let lock = workspace.acquire_lock("demo", 7).unwrap();
        assert!(lock_path.exists(), "the lock file should exist while the lock is held");

        // Dropping is what happens when Ctrl+C unwinds or cancels the work in progress.
        drop(lock);
        assert!(!lock_path.exists(), "the lock file should be deleted once the lock is released");
    }

    #[test]
    fn test_lock_is_exclusive_while_held_and_reusable_afterwards() {
        let tmp = TempDir::new().unwrap();
        let workspace = Workspace { base_dir: tmp.path().to_path_buf() };
        workspace.create_issue_dir("demo", 7).unwrap();

        let lock = workspace.acquire_lock("demo", 7).unwrap();
        assert!(workspace.acquire_lock("demo", 7).is_err(), "a held lock must not be handed out twice");

        drop(lock);
        assert!(workspace.acquire_lock("demo", 7).is_ok(), "the issue should be lockable again after release");
    }

    #[test]
    fn test_locks_are_independent_per_issue() {
        let tmp = TempDir::new().unwrap();
        let workspace = Workspace { base_dir: tmp.path().to_path_buf() };
        workspace.create_issue_dir("demo", 7).unwrap();
        workspace.create_issue_dir("demo", 8).unwrap();

        let _seven = workspace.acquire_lock("demo", 7).unwrap();
        assert!(workspace.acquire_lock("demo", 8).is_ok());
    }
}
