use super::*;
use crate::workspace::AgentConfig;
use tempfile::TempDir;

/// One real `claude --print --output-format stream-json --verbose` run: a tool
/// call, its result, and the closing prose.
const RECORDED_STREAM: &str = concat!(
    r#"{"type":"system","subtype":"init","cwd":"/tmp/ws","model":"claude-opus-5","session_id":"s1"}"#, "\n",
    r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"},"session_id":"s1"}"#, "\n",
    r#"{"type":"assistant","parent_tool_use_id":null,"message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"cargo test","description":"Run tests"}}]},"session_id":"s1"}"#, "\n",
    r#"{"type":"user","parent_tool_use_id":null,"message":{"role":"user","content":[{"tool_use_id":"t1","type":"tool_result","content":"test result: ok. 3 passed","is_error":false}]},"session_id":"s1"}"#, "\n",
    r#"{"type":"assistant","parent_tool_use_id":null,"message":{"role":"assistant","content":[{"type":"text","text":"Tests pass.\nCOMPLETED_DEVELOPMENT"}]},"session_id":"s1"}"#, "\n",
    r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":5405,"num_turns":2,"total_cost_usd":0.08,"result":"Tests pass.\nCOMPLETED_DEVELOPMENT","session_id":"s1"}"#, "\n",
);

async fn read(input: &str) -> (String, String) {
    let agent = Agent::new("claude --print {prompt}".to_string());
    let mut renderer = StreamRenderer::new();
    let mut audit: Vec<u8> = Vec::new();
    let mut source = input.as_bytes();

    agent.read_stdout(&mut source, &mut audit, &mut renderer).await.unwrap();

    (String::from_utf8(audit).unwrap(), renderer.into_transcript())
}

#[tokio::test]
async fn writes_a_readable_transcript_of_a_stream_json_run_to_the_audit_log() {
    let (audit, transcript) = read(RECORDED_STREAM).await;

    assert_eq!(
        audit,
        "* agent session started (claude-opus-5, cwd /tmp/ws)\n\
         > Bash(cargo test)\n\
         \x20 <- test result: ok. 3 passed\n\
         Tests pass.\nCOMPLETED_DEVELOPMENT\n\
         * agent run finished (5.4s, 2 turns, $0.0800)\n"
    );
    // The state machine only ever sees the agent's own words, not the JSON envelope.
    assert_eq!(transcript, "Tests pass.\nCOMPLETED_DEVELOPMENT\n");
    assert!(!audit.contains('\u{1b}'), "the audit log must stay free of ANSI codes");
}

#[tokio::test]
async fn passes_a_plain_text_agent_through_unchanged() {
    let (audit, transcript) = read("## Requirements Summary\n\n* one\nCOMPLETED_REQUIREMENTS\n").await;

    assert_eq!(audit, "## Requirements Summary\n\n* one\nCOMPLETED_REQUIREMENTS\n");
    assert_eq!(transcript, audit);
}

#[tokio::test]
async fn handles_output_that_never_ends_in_a_newline() {
    let (audit, transcript) = read("REVIEW_PASSED").await;

    assert_eq!(audit, "REVIEW_PASSED\n");
    assert_eq!(transcript, "REVIEW_PASSED\n");
}

#[tokio::test]
async fn handles_events_split_across_reads_and_windows_line_endings() {
    // The buffer is filled from 4 KiB reads, so a single event routinely arrives in pieces.
    let big = "x".repeat(9000);
    let input = format!(
        "{}\r\n{}\r\n",
        format!(r#"{{"type":"assistant","message":{{"content":[{{"type":"text","text":"{}"}}]}}}}"#, big),
        r#"{"type":"result","subtype":"success","is_error":false,"result":"ignored"}"#
    );

    let (_, transcript) = read(&input).await;
    assert_eq!(transcript, format!("{}\n", big));
}

/// A fake agent binary is needed here (rather than feeding text straight to
/// `read_stdout` like the tests above) because the rate-limit check runs
/// after the process exits, against the exit status - it's wiring across
/// `invoke_interactive`, not just the line renderer.
#[tokio::test]
async fn surfaces_a_usage_limit_as_a_rate_limit_error_instead_of_a_generic_failure() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    let script_path = tmp.path().join("fake_claude.sh");
    std::fs::write(
        &script_path,
        "#!/bin/sh\n\
         echo '{\"type\":\"result\",\"subtype\":\"error_during_execution\",\"is_error\":true,\"result\":\"You have hit your session limit \\u00b7 resets 11:59pm (UTC)\"}'\n\
         exit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let agent = Agent::new(format!("{} {{prompt}}", script_path.display()));
    let err = agent
        .invoke_interactive("do the thing", tmp.path().join("audit.log"), None)
        .await
        .unwrap_err();

    let rate_limit = err
        .downcast_ref::<crate::rate_limit::RateLimitError>()
        .unwrap_or_else(|| panic!("expected a RateLimitError, got: {}", err));
    assert!(rate_limit.message.contains("session limit"));
    assert!(rate_limit.reset_at.is_some(), "should have parsed the (UTC) reset time");
}

/// Exercises the real `claude` CLI end to end. Not run by default because it
/// needs the binary, network access and burns tokens:
/// `cargo test -- --ignored live_claude_run`
#[tokio::test]
#[ignore]
async fn live_claude_run_streams_the_conversation() {
    let tmp = TempDir::new().unwrap();
    // Goes through AgentConfig so the streaming flags are applied the same way
    // the orchestrator applies them.
    let config = AgentConfig {
        command_template: "claude --print --permission-mode acceptEdits {prompt}".to_string(),
        agents: [("default".to_string(), "claude".to_string())].into_iter().collect(),
        state_agents: Default::default(),
        stream_json: true,
        partial_messages: false,
    };
    let agent = Agent::new(config.get_command_for_state("ai-development"));

    let response = agent
        .invoke_interactive(
            "Use the Bash tool to run `echo hello`, then reply with exactly COMPLETED_DEVELOPMENT",
            tmp.path().join("audit.log"),
            Some(tmp.path().to_path_buf()),
        )
        .await
        .unwrap();

    assert!(response.contains("COMPLETED_DEVELOPMENT"), "unexpected response: {}", response);
    let audit = std::fs::read_to_string(tmp.path().join("audit.log")).unwrap();
    assert!(audit.contains("> Bash(echo hello)"), "tool call missing from audit log:\n{}", audit);
    assert!(audit.contains("agent run finished"), "run summary missing from audit log:\n{}", audit);
}
