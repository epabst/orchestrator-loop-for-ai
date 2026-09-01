use tokio::process::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::fs::File;
use std::process::Stdio;
use std::path::PathBuf;
use anyhow::{Result, anyhow};
use std::io::{self, Write};
use colored::Colorize;

use crate::agent_stream::StreamRenderer;

pub struct Agent {
    pub command_line: String,
}

impl Agent {
    pub fn new(command_line: String) -> Self {
        Self { command_line }
    }

    pub async fn invoke_interactive(
        &self, 
        prompt: &str, 
        audit_log_path: PathBuf, 
        current_dir: Option<PathBuf>
    ) -> Result<String> {
        // Support {prompt} placeholder: if present, inject prompt as a CLI arg
        // rather than writing to stdin. This is needed for tools like `claude --print {prompt}`.
        let use_prompt_arg = self.command_line.contains("{prompt}");
        let resolved_command = if use_prompt_arg {
            self.command_line.replace("{prompt}", prompt)
        } else {
            self.command_line.clone()
        };

        let parts: Vec<&str> = resolved_command.splitn(2, ' ').collect();
        let bin = parts[0];
        // Split remaining args carefully, preserving the injected prompt as one token
        let args: Vec<String> = if use_prompt_arg {
            // Re-split the original template args, substituting {prompt} as a single token
            let template_parts: Vec<&str> = self.command_line.splitn(2, ' ').collect();
            if template_parts.len() > 1 {
                // Split non-prompt args, then replace {prompt} token with the actual prompt
                template_parts[1]
                    .split_whitespace()
                    .map(|t| if t == "{prompt}" { prompt.to_string() } else { t.to_string() })
                    .collect()
            } else {
                vec![]
            }
        } else {
            if parts.len() > 1 {
                parts[1].split_whitespace().map(String::from).collect()
            } else {
                vec![]
            }
        };

        let mut command = Command::new(bin);
        command.args(&args);
        if let Some(dir) = current_dir {
            command.current_dir(dir);
        }
        command.env("GEMINI_CLI_TRUST_WORKSPACE", "true");

        let stdin_cfg = if use_prompt_arg { Stdio::null() } else { Stdio::piped() };

        let mut child = command
            .stdin(stdin_cfg)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn agent: {}", e))?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open stdout"))?;
        let mut audit_file = File::create(&audit_log_path).await?;

        println!("{} Sending prompt:\n{}", "DEBUG:".cyan(), prompt);
        audit_file.write_all(format!(">>> PROMPT:\n{}\n", prompt).as_bytes()).await?;

        if !use_prompt_arg {
            // Write prompt to stdin and close it
            let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open stdin"))?;
            stdin.write_all(prompt.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            drop(stdin);
        }

        let mut stdout_reader = BufReader::new(stdout);
        let mut renderer = StreamRenderer::new();

        let read_task = self.read_stdout(&mut stdout_reader, &mut audit_file, &mut renderer);

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                let _ = child.kill().await;
                return Err(anyhow!("Shutdown signal received. Agent process terminated."));
            }
            res = read_task => {
                res?;
                let status = child.wait().await?;
                if !status.success() {
                    return Err(anyhow!("Agent failed with status {}", status));
                }
            }
        }

        Ok(renderer.into_transcript())
    }

    /// Streams the agent's stdout line by line so the run is visible as it
    /// happens, rendering Claude's `stream-json` events into readable console
    /// output and leaving other agents' plain text alone.
    ///
    /// The audit log gets the same text without ANSI styling.
    async fn read_stdout<R, L>(
        &self,
        stdout: &mut R,
        audit_file: &mut L,
        renderer: &mut StreamRenderer,
    ) -> Result<()>
    where
        R: AsyncReadExt + Unpin,
        L: AsyncWriteExt + Unpin,
    {
        let mut out_buf = [0u8; 4096];
        let mut pending: Vec<u8> = Vec::new();

        loop {
            let n = stdout.read(&mut out_buf).await?;
            if n == 0 { break; } // EOF from agent
            pending.extend_from_slice(&out_buf[..n]);

            while let Some(newline) = pending.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = pending.drain(..=newline).collect();
                let line = String::from_utf8_lossy(&line[..newline]);
                Self::emit_line(line.trim_end_matches('\r'), audit_file, renderer).await?;
            }
        }

        // Trailing output with no final newline.
        if !pending.is_empty() {
            let line = String::from_utf8_lossy(&pending).to_string();
            Self::emit_line(line.trim_end_matches('\r'), audit_file, renderer).await?;
        }

        Ok(())
    }

    async fn emit_line<L>(
        line: &str,
        audit_file: &mut L,
        renderer: &mut StreamRenderer,
    ) -> Result<()>
    where
        L: AsyncWriteExt + Unpin,
    {
        for event in renderer.render_line(line) {
            io::stdout().write_all(event.colored().as_bytes())?;
            io::stdout().write_all(b"\n")?;
            io::stdout().flush()?;

            audit_file.write_all(event.text.as_bytes()).await?;
            audit_file.write_all(b"\n").await?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod agent_tests;
