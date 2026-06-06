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
        let parts: Vec<&str> = self.command_line.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("Empty command line"));
        }

        println!("{} Executing command: {}", "DEBUG:".cyan(), self.command_line);

        let mut command = Command::new(parts[0]);
        command.args(&parts[1..]);
        if let Some(dir) = current_dir {
            command.current_dir(dir);
        }
        command.env("GEMINI_CLI_TRUST_WORKSPACE", "true");
        
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn agent: {}", e))?;

        let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to open stdout"))?;
        let mut audit_file = File::create(&audit_log_path).await?;

        // Write initial prompt to agent and log it
        println!("{} Sending prompt:\n{}", "DEBUG:".cyan(), prompt);
        stdin.write_all(prompt.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        drop(stdin); // Close stdin to signal EOF to agent
        
        audit_file.write_all(format!(">>> PROMPT:\n{}\n", prompt).as_bytes()).await?;

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
