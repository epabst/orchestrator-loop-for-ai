mod config;
mod github;
mod workspace;
mod agent;
mod sdlc;

use clap::Parser;
use anyhow::Result;
use crate::config::SdlcConfig;
use crate::github::GithubClient;
use crate::workspace::Workspace;
use crate::sdlc::Orchestrator;
use std::fs;
use colored::Colorize;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// GitHub Personal Access Token
    #[arg(short, long, env = "GITHUB_TOKEN")]
    token: String,

    /// GitHub Repository Name
    #[arg(short, long)]
    repo: String,

    /// Path to SDLC config file
    #[arg(short, long, default_value = "sdlc_config.yaml")]
    config: String,

    /// Only run the loop once and exit
    #[arg(short, long)]
    once: bool,

    /// Only check the status and exit
    #[arg(short, long)]
    status: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config_yaml = fs::read_to_string(&args.config)?;
    let config = SdlcConfig::from_yaml(&config_yaml)?;

    let repo_name = args.repo
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .split('/')
        .last()
        .unwrap_or(&args.repo)
        .to_string();

    let mut github = GithubClient::new(args.token, String::new(), repo_name.clone())?;
    
    // Infer owner from token
    println!("  {} Inferring GitHub owner from token...", "INFO:".blue());
    let owner = github.get_current_user().await?;
    github.owner = owner;

    let workspace = Workspace::new()?;

    let orchestrator = Orchestrator::new(config, github, workspace);

    if args.status {
        orchestrator.status().await?;
        return Ok(());
    }

    orchestrator.run(args.once).await?;

    Ok(())
}
