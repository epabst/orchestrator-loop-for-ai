#[cfg(test)]
mod tests {
    use crate::config::SdlcConfig;

    #[test]
    fn test_initial_state_defaults_to_first_and_can_skip() {
        let base = |extra: &str| format!(r#"
states:
  - name: ai-requirements
    label: ai-requirements
  - name: ai-development
    label: ai-development
failure_label: f
human_help_label: h
lock_label: l
enabled_label: e
{}"#, extra);
        let c = SdlcConfig::from_yaml(&base("")).unwrap();
        assert_eq!(c.initial_state().name, "ai-requirements");
        let c = SdlcConfig::from_yaml(&base("initial_state: ai-development")).unwrap();
        assert_eq!(c.initial_state().name, "ai-development");
        let c = SdlcConfig::from_yaml(&base("initial_state: bogus")).unwrap();
        assert_eq!(c.initial_state().name, "ai-requirements");
    }

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
    fn test_native_handler_state_needs_no_prompt_or_keywords() {
        // A natively handled state runs orchestrator code, so it carries no
        // agent prompt and no keywords to match in an agent's reply.
        let yaml = r#"
states:
  - name: ai-pr-creation
    label: ai-pr-creation
    prompt: "Open a PR"
    next_state: ai-awaiting-pr-merge
    keywords:
      CHANGES_SUMMARIZED: ai-awaiting-pr-merge
  - name: ai-awaiting-pr-merge
    label: ai-awaiting-pr-merge
    native_handler: await-pr-merge
    next_state: ai-done
  - name: ai-done
    label: ai-done
    prompt: "Done"
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        let waiting = config.await_pr_merge_state().expect("await-pr-merge state");

        assert_eq!(waiting.label, "ai-awaiting-pr-merge");
        assert_eq!(waiting.prompt, "");
        assert!(waiting.keywords.is_empty());
        assert_eq!(config.await_pr_merge_label(), Some("ai-awaiting-pr-merge"));
        assert_eq!(config.label_after_pr_merge_wait(), "ai-done");
    }

    #[test]
    fn test_no_await_pr_merge_state_configured() {
        // Without the state, callers must see None rather than guess a label.
        let yaml = r#"
states:
  - name: ai-test
    label: ai-test
    prompt: "test"
    keywords:
      DONE: ai-done
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert!(config.await_pr_merge_state().is_none());
        assert_eq!(config.await_pr_merge_label(), None);
    }

    #[test]
    fn test_label_after_merge_wait_follows_next_state_label() {
        // next_state names a state; the issue gets that state's label.
        let yaml = r#"
states:
  - name: awaiting
    label: ai-awaiting-pr-merge
    native_handler: await-pr-merge
    next_state: shipped
  - name: shipped
    label: ai-shipped
    prompt: "Shipped"
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.label_after_pr_merge_wait(), "ai-shipped");
    }

    #[test]
    fn test_shipped_config_waits_for_the_merge_before_ai_done() {
        // The config this repo ships must route PR creation through the merge
        // wait, or a merged PR never produces a build of main.
        let yaml = std::fs::read_to_string("sdlc_config.yaml").expect("sdlc_config.yaml");
        let config = SdlcConfig::from_yaml(&yaml).unwrap();

        let pr_creation = config.get_state("ai-pr-creation").expect("ai-pr-creation state");
        assert_eq!(
            pr_creation.get_labels_for_keyword("CHANGES_SUMMARIZED"),
            vec!["ai-awaiting-pr-merge".to_string()]
        );

        let waiting = config.await_pr_merge_state().expect("await-pr-merge state");
        assert_eq!(waiting.label, "ai-awaiting-pr-merge");
        assert_eq!(config.label_after_pr_merge_wait(), "ai-done");
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

    #[test]
    fn test_auto_rebase_defaults_when_absent() {
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
        assert!(config.auto_rebase.enabled);
        assert_eq!(config.auto_rebase.main_branch, "main");
        assert_eq!(config.auto_rebase.publish_script, "publish-android-dev.sh");
        assert_eq!(config.auto_rebase.check_interval_secs, 300);
    }

    #[test]
    fn test_auto_rebase_can_be_configured() {
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
auto_rebase:
  enabled: false
  main_branch: master
  publish_script: publish-web.sh
  check_interval_secs: 60
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert!(!config.auto_rebase.enabled);
        assert_eq!(config.auto_rebase.main_branch, "master");
        assert_eq!(config.auto_rebase.publish_script, "publish-web.sh");
        assert_eq!(config.auto_rebase.check_interval_secs, 60);
    }

    #[test]
    fn test_auto_rebase_partial_config_keeps_defaults() {
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
auto_rebase:
  check_interval_secs: 900
"#;
        let config = SdlcConfig::from_yaml(yaml).unwrap();
        assert!(config.auto_rebase.enabled);
        assert_eq!(config.auto_rebase.main_branch, "main");
        assert_eq!(config.auto_rebase.check_interval_secs, 900);
    }
}
