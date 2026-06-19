mod config;
mod github;
mod workspace;
mod agent;
mod sdlc;
mod issue_context;
mod signal_handler;
mod readme_tests;

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
    repo: Option<String>,

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

    let mut github = if let Some(repo_arg) = &args.repo {
        let repo_name = repo_arg
            .trim_start_matches("https://github.com/")
            .trim_start_matches("http://github.com/")
            .split('/')
            .last()
            .unwrap_or(repo_arg)
            .to_string();
        
        let mut client = GithubClient::new(args.token, String::new(), repo_name.clone())?;
        let owner = client.get_current_user().await?;
        client.owner = owner;
        client
    } else {
        GithubClient::new(args.token, String::new(), String::new())?
    };
    
    // Infer owner if not set (for all-repos mode)
    if github.owner.is_empty() {
        println!("  {} Inferring GitHub owner from token...", "INFO:".blue());
        let owner = github.get_current_user().await?;
        github.owner = owner;
    }

    let workspace = Workspace::new()?;

    let mut orchestrator = Orchestrator::new(config, github, workspace);

    if args.status {
        orchestrator.status().await?;
        return Ok(());
    }

    orchestrator.run(args.once).await?;

    Ok(())
}
