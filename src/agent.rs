use tokio::process::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::fs::File;
use std::process::Stdio;
use std::path::PathBuf;
use anyhow::{Result, anyhow};
use std::io::{self, Write};
use colored::Colorize;

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

        println!("{} Executing command: {} (prompt via {})", "DEBUG:".cyan(), self.command_line,
            if use_prompt_arg { "arg" } else { "stdin" });

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
        let mut full_output = Vec::new();

        let read_task = self.read_stdout(&mut stdout_reader, &mut audit_file, &mut full_output);

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

        println!("{} Received response:\n{}", "DEBUG:".cyan(), String::from_utf8_lossy(&full_output));

        Ok(String::from_utf8_lossy(&full_output).to_string())
    }

    async fn read_stdout<R, L>(
        &self, 
        stdout: &mut R, 
        audit_file: &mut L, 
        full_output: &mut Vec<u8>
    ) -> Result<()> 
    where 
        R: AsyncReadExt + Unpin,
        L: AsyncWriteExt + Unpin,
    {
        let mut out_buf = [0u8; 1024];

        loop {
            let n = stdout.read(&mut out_buf).await?;
            if n == 0 { break; } // EOF from agent
            
            let data = &out_buf[..n];
            // Write to terminal (stdout)
            io::stdout().write_all(data)?;
            io::stdout().flush()?;
            
            // Write to audit log
            audit_file.write_all(data).await?;
            
            // Keep track of full output
            full_output.extend_from_slice(data);
        }
        Ok(())
    }
}
