use super::*;

fn render(renderer: &mut StreamRenderer, line: &str) -> Vec<String> {
    renderer.render_line(line).into_iter().map(|e| e.text).collect()
}

fn render_all(lines: &[&str]) -> (Vec<String>, String) {
    let mut renderer = StreamRenderer::new();
    let mut out = Vec::new();
    for line in lines {
        out.extend(render(&mut renderer, line));
    }
    (out, renderer.into_transcript())
}

#[test]
fn passes_plain_text_through_untouched() {
    let mut renderer = StreamRenderer::new();
    assert_eq!(render(&mut renderer, "COMPLETED_DESIGN"), vec!["COMPLETED_DESIGN"]);
    assert_eq!(render(&mut renderer, ""), vec![""]);
    assert_eq!(renderer.transcript(), "COMPLETED_DESIGN\n\n");
}

#[test]
fn passes_non_event_json_through_untouched() {
    let mut renderer = StreamRenderer::new();
    let line = r#"{"title": "not a stream event"}"#;
    assert_eq!(render(&mut renderer, line), vec![line]);
}

#[test]
fn renders_session_banner_from_init() {
    let (out, transcript) = render_all(&[
        r#"{"type":"system","subtype":"init","model":"claude-opus-5","cwd":"/tmp/ws"}"#,
    ]);
    assert_eq!(out, vec!["* agent session started (claude-opus-5, cwd /tmp/ws)"]);
    assert_eq!(transcript, "");
}

#[test]
fn renders_tool_call_with_its_key_argument() {
    let (out, _) = render_all(&[
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test","description":"Run tests"}}]}}"#,
    ]);
    assert_eq!(out, vec!["> Bash(cargo test)"]);
}

#[test]
fn collapses_whitespace_and_truncates_long_tool_arguments() {
    let long = "x".repeat(200);
    let line = format!(
        r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","name":"Bash","input":{{"command":"echo\n{}"}}}}]}}}}"#,
        long
    );
    let (out, _) = render_all(&[&line]);
    assert!(out[0].starts_with("> Bash(echo xxx"), "unexpected: {}", out[0]);
    assert!(out[0].ends_with("...)"), "unexpected: {}", out[0]);
    assert!(out[0].chars().count() < 140, "not truncated: {}", out[0]);
}

#[test]
fn falls_back_to_raw_input_when_no_key_argument() {
    let (out, _) = render_all(&[
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"TodoWrite","input":{"todos":[]}}]}}"#,
    ]);
    assert_eq!(out, vec![r#"> TodoWrite({"todos":[]})"#]);
}

#[test]
fn renders_assistant_prose_and_records_it_in_the_transcript() {
    let (out, transcript) = render_all(&[
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Implemented the parser.\nCOMPLETED_DEVELOPMENT"}]}}"#,
    ]);
    assert_eq!(out, vec!["Implemented the parser.\nCOMPLETED_DEVELOPMENT"]);
    assert!(transcript.contains("COMPLETED_DEVELOPMENT"));
}

#[test]
fn renders_thinking_without_polluting_the_transcript() {
    let (out, transcript) = render_all(&[
        r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"weighing options"}]}}"#,
    ]);
    assert_eq!(out, vec!["* thinking: weighing options"]);
    assert_eq!(transcript, "");
}

#[test]
fn renders_tool_results_truncated_to_a_few_lines() {
    let body = (1..=8).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\\n");
    let line = format!(
        r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","tool_use_id":"t1","content":"{}"}}]}}}}"#,
        body
    );
    let (out, transcript) = render_all(&[&line]);
    assert!(out[0].starts_with("  <- line 1"), "unexpected: {}", out[0]);
    assert!(out[0].contains("line 5"));
    assert!(!out[0].contains("line 6"));
    assert!(out[0].contains("... 3 more lines"));
    assert_eq!(transcript, "");
}

#[test]
fn marks_failed_tool_results_as_errors() {
    let events = StreamRenderer::new().render_line(
        r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"boom","is_error":true}]}}"#,
    );
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].style, Style::Error);
    assert_eq!(events[0].text, "  <- error: boom");
}

#[test]
fn reads_tool_results_delivered_as_content_blocks() {
    let (out, _) = render_all(&[
        r#"{"type":"user","message":{"content":[{"type":"tool_result","content":[{"type":"text","text":"hello"}]}]}}"#,
    ]);
    assert_eq!(out, vec!["  <- hello"]);
}

#[test]
fn labels_empty_tool_results() {
    let (out, _) = render_all(&[
        r#"{"type":"user","message":{"content":[{"type":"tool_result","content":""}]}}"#,
    ]);
    assert_eq!(out, vec!["  <- (no output)"]);
}

#[test]
fn indents_subagent_events() {
    let (out, _) = render_all(&[
        r#"{"type":"assistant","parent_tool_use_id":"toolu_1","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"src/main.rs"}}]}}"#,
    ]);
    assert_eq!(out, vec!["  | > Read(src/main.rs)"]);
}

#[test]
fn summarizes_a_successful_run() {
    let events = StreamRenderer::new().render_line(
        r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":12340,"num_turns":3,"total_cost_usd":0.0802615,"result":"done"}"#,
    );
    assert_eq!(events[0].text, "* agent run finished (12.3s, 3 turns, $0.0803)");
    assert_eq!(events[0].style, Style::Info);
}

#[test]
fn summarizes_a_failed_run() {
    let events = StreamRenderer::new().render_line(
        r#"{"type":"result","subtype":"error_max_turns","is_error":true,"num_turns":50}"#,
    );
    assert_eq!(events[0].text, "x agent run failed: error_max_turns (50 turns)");
    assert_eq!(events[0].style, Style::Error);
}

#[test]
fn keeps_the_result_text_when_nothing_else_reached_the_transcript() {
    let (_, transcript) = render_all(&[
        r#"{"type":"result","subtype":"success","is_error":false,"result":"REVIEW_PASSED"}"#,
    ]);
    assert_eq!(transcript, "REVIEW_PASSED\n");
}

#[test]
fn exposes_the_result_events_own_text_for_failure_inspection() {
    let mut renderer = StreamRenderer::new();
    assert_eq!(renderer.last_result_text(), None);

    renderer.render_line(
        r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"You've hit your session limit · resets 1:20pm (America/Denver)"}"#,
    );
    assert_eq!(
        renderer.last_result_text(),
        Some("You've hit your session limit \u{b7} resets 1:20pm (America/Denver)")
    );
}

#[test]
fn does_not_duplicate_prose_already_in_the_transcript() {
    let (_, transcript) = render_all(&[
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"PR_REQUESTED"}]}}"#,
        r#"{"type":"result","subtype":"success","is_error":false,"result":"PR_REQUESTED"}"#,
    ]);
    assert_eq!(transcript, "PR_REQUESTED\n");
}

#[test]
fn ignores_bookkeeping_events() {
    let (out, transcript) = render_all(&[
        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#,
    ]);
    assert!(out.is_empty());
    assert_eq!(transcript, "");
}

#[test]
fn streams_partial_text_without_repeating_it_on_completion() {
    let (out, transcript) = render_all(&[
        r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Review "}}}"#,
        r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"passed"}}}"#,
        r#"{"type":"stream_event","event":{"type":"content_block_stop","index":0}}"#,
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Review passed"}]}}"#,
    ]);
    assert_eq!(out, vec!["Review ", "passed"]);
    assert_eq!(transcript, "Review passed\n");
}

#[test]
fn ignores_non_text_deltas() {
    let (out, transcript) = render_all(&[
        r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"a\":"}}}"#,
    ]);
    assert!(out.is_empty());
    assert_eq!(transcript, "");
}

#[test]
fn plain_style_text_carries_no_ansi_codes() {
    let event = RenderedEvent { text: "hello".to_string(), style: Style::Plain };
    assert_eq!(event.colored(), "hello");
}
