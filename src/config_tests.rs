#[cfg(test)]
mod tests {
    use crate::config::SdlcConfig;

    #[test]
    fn test_config_parsing_new_format() {
        let yaml = r#"
states:
  - name: ai-design
    label: ai-design
    prompt: "Create design"
    keywords:
      KEYWORD1: ai-development
      KEYWORD2: ai-development, ai-human-help
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.states.len(), 1);
        assert_eq!(config.states[0].name, "ai-design");
        assert_eq!(config.failure_label, "ai-human-help");
    }

    #[test]
    fn test_backward_compatible_prompt_suffix() {
        let yaml = r#"
states:
  - name: ai-requirements
    label: ai-requirements
    prompt_suffix: "Test suffix"
    next_state: ai-design
    keywords:
      success: COMPLETED
    doc_file: reqs.md
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.states.len(), 1);
        assert_eq!(config.states[0].prompt, "Test suffix");
    }

    #[test]
    fn test_keyword_maps_to_single_label() {
        let yaml = r#"
states:
  - name: ai-test
    label: ai-test
    prompt: "test"
    keywords:
      SUCCESS: ai-next
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        let state = &config.states[0];

        let labels = state.get_labels_for_keyword("SUCCESS");
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0], "ai-next");
    }

    #[test]
    fn test_keyword_maps_to_multiple_labels() {
        let yaml = r#"
states:
  - name: ai-test
    label: ai-test
    prompt: "test"
    keywords:
      MULTI: ai-review, ai-human-help
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        let state = &config.states[0];

        let labels = state.get_labels_for_keyword("MULTI");
        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&"ai-review".to_string()));
        assert!(labels.contains(&"ai-human-help".to_string()));
    }

    #[test]
    fn test_get_state() {
        let yaml = r#"
states:
  - name: ai-requirements
    label: ai-requirements
    prompt: "Test prompt"
    next_state: ai-design
    keywords:
      success: COMPLETED
    doc_file: reqs.md
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        let state = config.get_state("ai-requirements").unwrap();
        assert_eq!(state.name, "ai-requirements");
    }

    #[test]
    fn test_multi_label_keyword_with_spaces() {
        let yaml = r#"
states:
  - name: ai-complex
    label: ai-complex
    prompt: "test"
    keywords:
      MULTI_LABEL: ai-phase1, ai-phase2, ai-human-help
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        let state = &config.states[0];

        let labels = state.get_labels_for_keyword("MULTI_LABEL");
        assert_eq!(labels.len(), 3);
        assert!(labels.contains(&"ai-phase1".to_string()));
        assert!(labels.contains(&"ai-phase2".to_string()));
        assert!(labels.contains(&"ai-human-help".to_string()));
    }

    #[test]
    fn test_all_configs_have_failure_label() {
        let yaml = r#"
states:
  - name: ai-test
    label: ai-test
    prompt: "test"
    keywords:
      DONE: ai-next
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.failure_label, "ai-human-help");
    }

    #[test]
    fn test_realistic_multi_label_workflow() {
        // Test a realistic scenario with multi-label keywords for more flexible workflows
        let yaml = r#"
states:
  - name: content-review
    label: content-review
    prompt: "Review the content"
    keywords:
      APPROVED: approved
      NEEDS_REVISION: needs-revision, content-review
      ESCALATE: escalated, human-review
failure_label: review-failed
human_help_label: human-review
lock_label: in-progress
enabled_label: enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        let state = &config.states[0];

        // Single label case
        let approved_labels = state.get_labels_for_keyword("APPROVED");
        assert_eq!(approved_labels.len(), 1);
        assert_eq!(approved_labels[0], "approved");

        // Multi-label case: NEEDS_REVISION maps to two labels
        let revision_labels = state.get_labels_for_keyword("NEEDS_REVISION");
        assert_eq!(revision_labels.len(), 2);
        assert!(revision_labels.contains(&"needs-revision".to_string()));
        assert!(revision_labels.contains(&"content-review".to_string()));

        // Triple-label case: ESCALATE maps to two labels
        let escalate_labels = state.get_labels_for_keyword("ESCALATE");
        assert_eq!(escalate_labels.len(), 2);
        assert!(escalate_labels.contains(&"escalated".to_string()));
        assert!(escalate_labels.contains(&"human-review".to_string()));

        // Verify failure_label is configured
        assert_eq!(config.failure_label, "review-failed");
    }

    #[test]
    fn test_pr_defined_keyword_defaults() {
        // Test that pr_defined_keyword defaults to CHANGES_SUMMARIZED
        let yaml = r#"
states:
  - name: ai-test
    label: ai-test
    prompt: "test"
    keywords:
      DONE: ai-next
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.pr_defined_keyword, "CHANGES_SUMMARIZED");
    }

    #[test]
    fn test_pr_defined_keyword_custom() {
        // Test that pr_defined_keyword can be customized
        let yaml = r#"
states:
  - name: ai-test
    label: ai-test
    prompt: "test"
    keywords:
      DONE: ai-next
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
pr_defined_keyword: CUSTOM_CLEANUP_MARKER
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.pr_defined_keyword, "CUSTOM_CLEANUP_MARKER");
    }

    #[test]
    fn test_merge_conflict_keyword_defaults() {
        // Test that merge_conflict_keyword defaults to MERGE_RESOLVED
        let yaml = r#"
states:
  - name: ai-test
    label: ai-test
    prompt: "test"
    keywords:
      DONE: ai-next
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.merge_conflict_keyword, "MERGE_RESOLVED");
    }

    #[test]
    fn test_merge_conflict_keyword_custom() {
        // Test that merge_conflict_keyword can be customized
        let yaml = r#"
states:
  - name: ai-test
    label: ai-test
    prompt: "test"
    keywords:
      DONE: ai-next
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
merge_conflict_keyword: CONFLICTS_FIXED
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.merge_conflict_keyword, "CONFLICTS_FIXED");
    }

    #[test]
    fn test_both_keywords_configured() {
        // Test that both cleanup and merge conflict keywords can be configured together
        let yaml = r#"
states:
  - name: ai-test
    label: ai-test
    prompt: "test"
    keywords:
      DONE: ai-next
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
pr_defined_keyword: DONE_WITH_PR
merge_conflict_keyword: RESOLUTION_COMPLETE
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.pr_defined_keyword, "DONE_WITH_PR");
        assert_eq!(config.merge_conflict_keyword, "RESOLUTION_COMPLETE");
    }
}
