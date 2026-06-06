#[cfg(test)]
mod tests {
    use crate::workspace::{AgentConfig};
    use std::collections::HashMap;

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
}
