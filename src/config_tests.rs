#[cfg(test)]
mod tests {
    use crate::config::SdlcConfig;

    #[test]
    fn test_config_parsing() {
        let yaml = r#"
states:
  - name: ai-requirements
    label: ai-requirements
    prompt_suffix: "Test suffix"
    next_state: ai-design
    keywords:
      success: COMPLETED
      failure: FAILED
    doc_file: reqs.md
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.states.len(), 1);
        assert_eq!(config.states[0].name, "ai-requirements");
        assert_eq!(config.human_help_label, "ai-human-help");
    }

    #[test]
    fn test_get_state() {
        let yaml = r#"
states:
  - name: ai-requirements
    label: ai-requirements
    prompt_suffix: "Test suffix"
    next_state: ai-design
    keywords:
      success: COMPLETED
    doc_file: reqs.md
human_help_label: h
lock_label: l
enabled_label: e
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        let state = config.get_state("ai-requirements").unwrap();
        assert_eq!(state.name, "ai-requirements");
    }
}
