use colored::Colorize;
use serde_json::Value;

/// How the console should tint a rendered chunk. The audit log stores the plain
/// text, so colors never leak into the saved transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// The agent's own prose - shown exactly as it arrived.
    Plain,
    /// Supporting detail (thinking, tool results, session banner).
    Dim,
    /// A tool the agent decided to run.
    Tool,
    /// A failed tool result or a failed run.
    Error,
    /// Run-level status such as the closing summary.
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEvent {
    pub text: String,
    pub style: Style,
}

impl RenderedEvent {
    fn new(text: impl Into<String>, style: Style) -> Self {
        Self { text: text.into(), style }
    }

    /// The console form, with ANSI styling applied.
    pub fn colored(&self) -> String {
        match self.style {
            Style::Plain => self.text.clone(),
            Style::Dim => self.text.dimmed().to_string(),
            Style::Tool => self.text.green().bold().to_string(),
            Style::Error => self.text.red().to_string(),
            Style::Info => self.text.cyan().to_string(),
        }
    }
}

const TOOL_INPUT_MAX_CHARS: usize = 120;
const TOOL_RESULT_MAX_LINES: usize = 5;
const TOOL_RESULT_MAX_CHARS: usize = 160;

/// Turns an agent's stdout into something a human can follow while the run is
/// still in progress.
///
/// Claude Code's `--output-format stream-json` emits one JSON object per line;
/// each one becomes a short line of console output (tool calls, tool results,
/// prose) instead of an opaque blob that only appears once the agent exits.
/// Lines that are not recognized stream events pass through untouched, so
/// plain-text agents (`gemini`, `claude --print` without the flag) keep working.
///
/// Alongside the display text the renderer accumulates a *transcript*: just the
/// agent's prose, which is what the state machine scans for completion keywords.
pub struct StreamRenderer {
    transcript: String,
    /// Text for the in-flight assistant message already appeared via deltas, so
    /// the completed message should not repeat it.
    streamed_text: bool,
    saw_stream_event: bool,
}

impl StreamRenderer {
    pub fn new() -> Self {
        Self { transcript: String::new(), streamed_text: false, saw_stream_event: false }
    }

    /// The agent's prose only - the text completion keywords are matched against.
    #[allow(dead_code)]
    pub fn transcript(&self) -> &str {
        &self.transcript
    }

    pub fn into_transcript(self) -> String {
        self.transcript
    }

    /// Renders one line of agent stdout (newline already stripped).
    pub fn render_line(&mut self, line: &str) -> Vec<RenderedEvent> {
        let event: Option<Value> = if line.trim_start().starts_with('{') {
            serde_json::from_str(line).ok()
        } else {
            None
        };

        let Some(event) = event.filter(|v| v.get("type").and_then(Value::as_str).is_some()) else {
            // Not a stream event: this agent speaks plain text.
            self.transcript.push_str(line);
            self.transcript.push('\n');
            return vec![RenderedEvent::new(line, Style::Plain)];
        };

        let indent = if event.get("parent_tool_use_id").and_then(Value::as_str).is_some() {
            "  | "
        } else {
            ""
        };

        let rendered = match event["type"].as_str().unwrap_or_default() {
            "system" => self.render_system(&event),
            "assistant" => self.render_assistant(&event),
            "user" => self.render_user(&event),
            "stream_event" => self.render_stream_event(&event),
            "result" => self.render_result(&event),
            // Bookkeeping events (rate limits, hooks, ...) are noise here.
            _ => vec![],
        };

        if indent.is_empty() {
            rendered
        } else {
            rendered.into_iter().map(|e| RenderedEvent::new(prefix_lines(&e.text, indent), e.style)).collect()
        }
    }

    fn render_system(&self, event: &Value) -> Vec<RenderedEvent> {
        if event["subtype"].as_str() != Some("init") {
            return vec![];
        }
        let model = event["model"].as_str().unwrap_or("unknown model");
        let mut line = format!("* agent session started ({}", model);
        if let Some(cwd) = event["cwd"].as_str() {
            line.push_str(&format!(", cwd {}", cwd));
        }
        line.push(')');
        vec![RenderedEvent::new(line, Style::Dim)]
    }

    fn render_assistant(&mut self, event: &Value) -> Vec<RenderedEvent> {
        let mut out = Vec::new();
        for block in content_blocks(&event["message"]) {
            match block["type"].as_str().unwrap_or_default() {
                "text" => {
                    let text = block["text"].as_str().unwrap_or_default();
                    if text.is_empty() {
                        continue;
                    }
                    if self.streamed_text {
                        // Already shown token by token; keep the transcript single-copy.
                        continue;
                    }
                    self.transcript.push_str(text);
                    self.transcript.push('\n');
                    out.push(RenderedEvent::new(text, Style::Plain));
                }
                "thinking" => {
                    let thinking = block["thinking"].as_str().unwrap_or_default();
                    if !thinking.trim().is_empty() {
                        out.push(RenderedEvent::new(format!("* thinking: {}", thinking.trim()), Style::Dim));
                    }
                }
                "tool_use" => {
                    let name = block["name"].as_str().unwrap_or("tool");
                    out.push(RenderedEvent::new(
                        format!("> {}({})", name, summarize_tool_input(&block["input"])),
                        Style::Tool,
                    ));
                }
                _ => {}
            }
        }
        self.streamed_text = false;
        out
    }

    fn render_user(&mut self, event: &Value) -> Vec<RenderedEvent> {
        let mut out = Vec::new();
        for block in content_blocks(&event["message"]) {
            if block["type"].as_str() != Some("tool_result") {
                continue;
            }
            let is_error = block["is_error"].as_bool().unwrap_or(false);
            let body = tool_result_text(&block["content"]);
            let style = if is_error { Style::Error } else { Style::Dim };
            let label = if is_error { "  <- error: " } else { "  <- " };
            let summary = truncate_block(&body);
            let summary = if summary.trim().is_empty() { "(no output)".to_string() } else { summary };
            out.push(RenderedEvent::new(format!("{}{}", label, summary), style));
        }
        out
    }

    fn render_stream_event(&mut self, event: &Value) -> Vec<RenderedEvent> {
        let inner = &event["event"];
        match inner["type"].as_str().unwrap_or_default() {
            "content_block_delta" => {
                let delta = &inner["delta"];
                if delta["type"].as_str() != Some("text_delta") {
                    return vec![];
                }
                let text = delta["text"].as_str().unwrap_or_default();
                if text.is_empty() {
                    return vec![];
                }
                self.streamed_text = true;
                self.saw_stream_event = true;
                self.transcript.push_str(text);
                vec![RenderedEvent::new(text, Style::Plain)]
            }
            "content_block_stop" if self.saw_stream_event => {
                self.transcript.push('\n');
                vec![]
            }
            _ => vec![],
        }
    }

    fn render_result(&mut self, event: &Value) -> Vec<RenderedEvent> {
        // The result text repeats the last assistant message, so it only feeds
        // the transcript when nothing else did (e.g. an agent that streams
        // nothing but the final answer).
        if self.transcript.trim().is_empty() {
            if let Some(text) = event["result"].as_str() {
                self.transcript.push_str(text);
                self.transcript.push('\n');
            }
        }

        let mut parts = Vec::new();
        if let Some(ms) = event["duration_ms"].as_f64() {
            parts.push(format!("{:.1}s", ms / 1000.0));
        }
        if let Some(turns) = event["num_turns"].as_i64() {
            parts.push(format!("{} turns", turns));
        }
        if let Some(cost) = event["total_cost_usd"].as_f64() {
            parts.push(format!("${:.4}", cost));
        }
        let detail = if parts.is_empty() { String::new() } else { format!(" ({})", parts.join(", ")) };

        if event["is_error"].as_bool().unwrap_or(false) {
            let subtype = event["subtype"].as_str().unwrap_or("error");
            vec![RenderedEvent::new(format!("x agent run failed: {}{}", subtype, detail), Style::Error)]
        } else {
            vec![RenderedEvent::new(format!("* agent run finished{}", detail), Style::Info)]
        }
    }
}

impl Default for StreamRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// `message.content` is a list of blocks, but a plain string is also valid.
fn content_blocks(message: &Value) -> Vec<Value> {
    match &message["content"] {
        Value::Array(blocks) => blocks.clone(),
        Value::String(text) => vec![serde_json::json!({"type": "text", "text": text})],
        _ => vec![],
    }
}

/// A tool result's content is either a string or a list of text blocks.
fn tool_result_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Picks the field that best identifies what a tool call is about, mirroring how
/// an interactive session labels the call.
fn summarize_tool_input(input: &Value) -> String {
    const PREFERRED: [&str; 8] =
        ["command", "file_path", "path", "pattern", "url", "query", "prompt", "description"];

    let summary = PREFERRED
        .iter()
        .find_map(|key| input.get(*key).and_then(Value::as_str))
        .map(str::to_string)
        .unwrap_or_else(|| match input {
            Value::Null => String::new(),
            other => other.to_string(),
        });

    truncate_chars(&summary.split_whitespace().collect::<Vec<_>>().join(" "), TOOL_INPUT_MAX_CHARS)
}

/// Keeps a tool result to a glanceable size.
fn truncate_block(text: &str) -> String {
    let lines: Vec<&str> = text.trim_end().lines().collect();
    let mut out: Vec<String> = lines
        .iter()
        .take(TOOL_RESULT_MAX_LINES)
        .map(|l| truncate_chars(l, TOOL_RESULT_MAX_CHARS))
        .collect();
    if lines.len() > TOOL_RESULT_MAX_LINES {
        out.push(format!("... {} more lines", lines.len() - TOOL_RESULT_MAX_LINES));
    }
    out.join("\n     ")
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max).collect();
    format!("{}...", kept)
}

fn prefix_lines(text: &str, prefix: &str) -> String {
    text.lines().map(|l| format!("{}{}", prefix, l)).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
#[path = "agent_stream_tests.rs"]
mod agent_stream_tests;
