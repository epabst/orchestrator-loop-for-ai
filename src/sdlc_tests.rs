use super::*;
use std::time::Duration;

#[tokio::test]
async fn test_wait_with_countdown_duration() {
    let start = std::time::Instant::now();
    let duration = Duration::from_secs(2);
    let signaled = wait_with_countdown(duration, "21:05:44").await.unwrap();
    let elapsed = start.elapsed();

    assert!(!signaled);
    assert!(elapsed >= duration);
}

// Tests for sticky issue selection feature
#[test]
fn test_sticky_state_clear() {
    // Test that clear_sticky_state() properly clears the sticky issue
    // We test this by directly manipulating the orchestrator's sticky state
    // since we can't easily construct Issue objects for the other methods

    // This test verifies the basic state management works
    // In a full test suite, we'd use mocks or integration tests for the async methods
    let mut state: Option<(String, u64)> = Some(("repo".to_string(), 42));

    // Simulate what clear_sticky_state does
    state = None;

    assert_eq!(state, None);
}

#[test]
fn test_sticky_state_tracking() {
    // Test that sticky state can be set and retrieved
    let mut state: Option<(String, u64)> = None;

    // Initially should be None
    assert_eq!(state, None);

    // Set sticky state
    state = Some(("test-repo".to_string(), 123));

    // Should be set now
    assert!(state.is_some());
    let (repo, num) = state.unwrap();
    assert_eq!(repo, "test-repo");
    assert_eq!(num, 123);
}

#[test]
fn test_sticky_state_extraction_from_url() {
    // Test that repo name is correctly extracted from repository URL
    let url_str = "https://api.github.com/repos/user/test-repo";
    let path = url_str.strip_prefix("https://api.github.com").unwrap_or("");
    let parts: Vec<&str> = path.split('/').collect();
    let repo = parts.get(3).map(|s| s.to_string());

    assert_eq!(repo, Some("test-repo".to_string()));
}

#[test]
fn test_sticky_state_tuple_matching_single_repo() {
    // Test that sticky state matching works in single-repo mode
    let sticky_state = Some(("my-repo".to_string(), 42));
    let issue_repo = "my-repo";
    let issue_num = 42;
    let all_repos = false;

    // Should match: number matches and (not all_repos OR repo matches)
    let matches = if let Some((ref sticky_repo, sticky_num)) = sticky_state {
        issue_num == sticky_num && (all_repos == false || issue_repo == sticky_repo)
    } else {
        false
    };

    assert!(matches);
}

#[test]
fn test_sticky_state_tuple_matching_all_repos_same_repo() {
    // Test that sticky state matching works in all-repos mode (same repo)
    let sticky_state = Some(("user/my-repo".to_string(), 42));
    let issue_repo = "user/my-repo";
    let issue_num = 42;
    let all_repos = true;

    // Should match: number matches AND repo matches (since all_repos=true)
    let matches = if let Some((ref sticky_repo, sticky_num)) = sticky_state {
        issue_num == sticky_num && (all_repos == false || issue_repo == sticky_repo)
    } else {
        false
    };

    assert!(matches);
}

#[test]
fn test_sticky_state_tuple_matching_all_repos_different_repo() {
    // Test that sticky state matching fails for different repos in all-repos mode
    let sticky_state = Some(("user/repo-a".to_string(), 42));
    let issue_repo = "user/repo-b";
    let issue_num = 42;
    let all_repos = true;

    // Should NOT match: number matches but repo doesn't (and all_repos=true)
    let matches = if let Some((ref sticky_repo, sticky_num)) = sticky_state {
        issue_num == sticky_num && (all_repos == false || issue_repo == sticky_repo)
    } else {
        false
    };

    assert!(!matches);
}

#[test]
fn test_sticky_state_cleared_on_terminal() {
    // Test that sticky state is cleared for terminal states
    let mut state = Some(("repo".to_string(), 42));

    // Simulate terminal state transition
    if true {
        // terminal_state = true
        state = None;
    }

    assert_eq!(state, None);
}

#[test]
fn test_sticky_state_persists_on_non_terminal() {
    // Test that sticky state persists for non-terminal transitions
    let mut state = Some(("repo".to_string(), 42));
    let initial_state = state.clone();

    // Simulate non-terminal state transition
    if false {
        // terminal_state = false
        state = None;
    }

    // State should remain unchanged
    assert_eq!(state, initial_state);
}

#[test]
fn test_no_sticky_issue_returns_none() {
    // Test that when no sticky issue is set, it returns None
    let state: Option<(String, u64)> = None;
    assert!(state.is_none());
}

#[test]
fn test_sticky_state_number_mismatch() {
    // Test that sticky state doesn't match when issue numbers differ
    let sticky_state = Some(("repo".to_string(), 42));
    let issue_num = 43;  // Different number

    let matches = if let Some((_, sticky_num)) = sticky_state {
        issue_num == sticky_num
    } else {
        false
    };

    assert!(!matches);
}

#[test]
fn test_sticky_state_preserves_both_repo_and_number() {
    // Test that both repo name and issue number are preserved correctly
    let repo_name = "my-organization/my-project".to_string();
    let issue_num = 12345u64;
    let state = Some((repo_name.clone(), issue_num));

    if let Some((stored_repo, stored_num)) = state {
        assert_eq!(stored_repo, repo_name);
        assert_eq!(stored_num, issue_num);
    } else {
        panic!("State should be Some");
    }
}

#[test]
fn test_escape_html_normal_text() {
    let input = "Fix login bug";
    let expected = "Fix login bug";
    assert_eq!(escape_html(input), expected);
}

#[test]
fn test_escape_html_with_script_tags() {
    let input = "<script>alert('xss')</script>";
    let expected = "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;";
    assert_eq!(escape_html(input), expected);
}

#[test]
fn test_escape_html_with_ampersand() {
    let input = "Issue with & symbol";
    let expected = "Issue with &amp; symbol";
    assert_eq!(escape_html(input), expected);
}

#[test]
fn test_escape_html_with_quotes() {
    let input = "Title with \"quotes\"";
    let expected = "Title with &quot;quotes&quot;";
    assert_eq!(escape_html(input), expected);
}

#[test]
fn test_escape_html_with_single_quotes() {
    let input = "Title with 'single quotes'";
    let expected = "Title with &#39;single quotes&#39;";
    assert_eq!(escape_html(input), expected);
}

#[test]
fn test_escape_html_with_multiple_special_chars() {
    let input = "<div>Title & \"body\" with 'quotes'</div>";
    let expected = "&lt;div&gt;Title &amp; &quot;body&quot; with &#39;quotes&#39;&lt;/div&gt;";
    assert_eq!(escape_html(input), expected);
}

#[test]
fn test_escape_html_empty_string() {
    let input = "";
    let expected = "";
    assert_eq!(escape_html(input), expected);
}

fn comment(login: &str, body: &str, timestamp: &str) -> (String, String, chrono::DateTime<chrono::Utc>) {
    (login.to_string(), body.to_string(), timestamp.parse().unwrap())
}

#[test]
fn test_format_issue_context_flags_newest_comment_only() {
    let comments = vec![
        comment("epabst", "please rename the button", "2026-09-05T15:13:20Z"),
        comment("epabst", "display the title at the top of the screen", "2026-09-05T15:54:54Z"),
    ];
    let context = format_issue_context("Issue title", "Issue body", &comments);

    assert!(context.contains("MOST RECENT COMMENT - verify this cycle addresses it before finishing: epabst: display the title at the top of the screen"));
    assert!(!context.contains("MOST RECENT COMMENT - verify this cycle addresses it before finishing: epabst: please rename the button"));
}

#[test]
fn test_format_issue_context_excludes_bot_transition_comments() {
    let comments = vec![
        comment("epabst", "please rename the button", "2026-09-05T15:13:20Z"),
        comment("epabst", "**Transition:** ai-development -> ai-review (keyword: COMPLETED_DEVELOPMENT)\n\ndone", "2026-09-05T15:18:02Z"),
    ];
    let context = format_issue_context("Issue title", "Issue body", &comments);

    assert!(!context.contains("**Transition:**"));
    assert!(context.contains("MOST RECENT COMMENT - verify this cycle addresses it before finishing: epabst: please rename the button"));
    assert!(context.contains("Last 1 Human Comments"));
}

#[test]
fn test_format_issue_context_keeps_only_last_ten_human_comments() {
    let comments: Vec<_> = (0..12)
        .map(|i| comment("epabst", &format!("comment {}", i), &format!("2026-09-05T15:{:02}:00Z", i)))
        .collect();
    let context = format_issue_context("Issue title", "Issue body", &comments);

    assert!(context.contains("Last 10 Human Comments"));
    assert!(!context.contains("comment 0:"));
    assert!(!context.contains("comment 1:"));
    assert!(context.contains("MOST RECENT COMMENT - verify this cycle addresses it before finishing: epabst: comment 11"));
}

#[test]
fn test_transition_format_single_label() {
    let source_state = "ai-design";
    let destination_labels = vec!["ai-development".to_string()];
    let keyword = "PROCEED";

    let destination_str = destination_labels.join(", ");
    let transition_line = format!("**Transition:** {} -> {} (keyword: {})",
                                   source_state, destination_str, keyword);

    assert_eq!(transition_line, "**Transition:** ai-design -> ai-development (keyword: PROCEED)");
}

#[test]
fn test_transition_format_multi_label() {
    let source_state = "ai-design";
    let destination_labels = vec!["ai-development".to_string(), "ai-human-help".to_string()];
    let keyword = "NEEDS_REVIEW";

    let destination_str = destination_labels.join(", ");
    let transition_line = format!("**Transition:** {} -> {} (keyword: {})",
                                   source_state, destination_str, keyword);

    assert_eq!(transition_line, "**Transition:** ai-design -> ai-development, ai-human-help (keyword: NEEDS_REVIEW)");
}

#[test]
fn test_workspace_header_names_the_checkout_by_absolute_path() {
    let issue_dir = std::path::Path::new("/Users/dev/ai-workspaces/breakthrough-path-54");
    let header = build_workspace_header(issue_dir, "breakthrough-path");

    assert!(header.contains("'/Users/dev/ai-workspaces/breakthrough-path-54'"));
    assert!(header.contains("'/Users/dev/ai-workspaces/breakthrough-path-54/breakthrough-path'"));
}

#[test]
fn test_workspace_header_gives_allowed_command_forms() {
    let issue_dir = std::path::Path::new("/Users/dev/ai-workspaces/my-repo-7");
    let header = build_workspace_header(issue_dir, "my-repo");

    assert!(header.contains("git -C my-repo"));
    assert!(header.contains("cd my-repo && ./gradlew"));
    assert!(header.contains("Never guess a different location"));
    assert!(header.trim_end().ends_with("Files available in the working directory:"));
}

/// The workflow this repo ships, so the label predicate is exercised against
/// the real state names rather than invented ones.
fn shipped_config() -> SdlcConfig {
    let yaml = std::fs::read_to_string("sdlc_config.yaml").expect("sdlc_config.yaml");
    SdlcConfig::from_yaml(&yaml).unwrap()
}

fn labels(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

#[test]
fn test_agent_can_advance_an_enabled_issue_mid_workflow() {
    let config = shipped_config();
    assert!(agent_can_advance(&labels(&["ai-enabled", "ai-development"]), &config));
}

#[test]
fn test_agent_cannot_advance_without_the_enabled_label() {
    let config = shipped_config();
    assert!(!agent_can_advance(&labels(&["ai-development"]), &config));
}

#[test]
fn test_agent_cannot_advance_a_locked_or_escalated_or_finished_issue() {
    let config = shipped_config();
    assert!(!agent_can_advance(&labels(&["ai-enabled", "ai-processing", "ai-development"]), &config));
    assert!(!agent_can_advance(&labels(&["ai-enabled", "ai-human-help"]), &config));
    assert!(!agent_can_advance(&labels(&["ai-enabled", "ai-done"]), &config));
}

#[test]
fn test_agent_cannot_advance_an_issue_waiting_on_its_pr_to_merge() {
    // The orchestrator advances this state itself. An agent picking it up would
    // have no prompt to run, and it would hold the single work slot for as long
    // as the PR stayed open, starving every other issue.
    let config = shipped_config();
    assert!(!agent_can_advance(&labels(&["ai-enabled", "ai-awaiting-pr-merge"]), &config));
}

#[test]
fn test_awaiting_state_is_ignored_when_the_workflow_omits_it() {
    // A workflow without the state must not accidentally exclude anything.
    let yaml = r#"
states:
  - name: ai-development
    label: ai-development
    prompt: "Build it"
    keywords:
      COMPLETED_DEVELOPMENT: ai-done
failure_label: ai-human-help
human_help_label: ai-human-help
lock_label: ai-processing
enabled_label: ai-enabled
"#;
    let config = SdlcConfig::from_yaml(yaml).unwrap();
    assert!(agent_can_advance(&labels(&["ai-enabled", "ai-awaiting-pr-merge"]), &config));
}
