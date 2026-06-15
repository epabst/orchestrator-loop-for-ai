# Orchestrator Loop for AI

A Rust-based CLI tool that manages an AI-driven software development lifecycle (SDLC) using GitHub issues as the state controller.

## Features

- **GitHub-First Workflow:** Uses GitHub labels and comments to drive the SDLC.
- **Automated SDLC Stages:**
  - `ai-requirements`: Requirements gathering and spec document creation.
  - `ai-design`: Technical design document creation.
  - `ai-development`: Test-driven development.
  - `ai-review`: Automated code review.
  - `ai-pr-creation`: Automated Pull Request creation.
  - `ai-done`: Completion state.
- **Concurrency Safety:** Uses both GitHub labels (`ai-processing`) and local file locking to ensure only one instance processes an issue at a time.
- **Human-in-the-loop:** Easily pause for human help by adding an `ai-human-help` label.
- **Configurable Agents:** Map different SDLC states to different AI agents (defaulting to `agy`).
- **Flexible Prompt Delivery:** Supports both stdin-based agents (e.g., `gemini`) and CLI-argument-based agents (e.g., `agy`) via the `{prompt}` placeholder.
- **Cross-platform:** Works on Windows, Mac, and Linux.

## Prerequisites

- Rust (installed via [rustup](https://rustup.rs/))
- GITHUB_TOKEN environment variable set with the necessary permissions.
  - **Best Practice**: Use a **Fine-grained PAT**.
  - To generate: Go to [GitHub Settings](https://github.com/settings/tokens) -> "Personal access tokens" -> "Fine-grained tokens" -> "Generate new token".
  - Select the target repository, and grant the following repository permissions:
    - **Issues**: Read and write (Mandatory for the orchestrator to list and manage issues)
    - **Metadata**: Read-only
    - **Contents**: Read and write
- An AI Agent CLI installed and configured. The default is [`agy`](https://github.com/antgravity/agy).

## Installation

```bash
cargo install --path .
```

## Configuration

### SDLC Configuration (`sdlc_config.yaml`)

Defines the states, labels, prompt suffixes, and keywords for transitions.

### Agent Configuration (`~/ai-workspaces/config.yaml`)

Maps states to agent commands. Created automatically on first run with `agy` as the default.

```yaml
command_template: agy --dangerously-skip-permissions --print {prompt}
agents:
  default: agy
state_agents: {}
```

#### Using a different agent

The `command_template` controls how the agent is invoked. The special `{prompt}` token is replaced with the full prompt at runtime as a single argument.

**For `agy` (default — prompt passed as CLI argument):**
```yaml
command_template: agy --dangerously-skip-permissions --print {prompt}
agents:
  default: agy
state_agents: {}
```

**For `gemini` (prompt passed via stdin):**
```yaml
command_template: gemini
agents:
  default: gemini
state_agents: {}
```

**Per-state agent overrides:**
```yaml
command_template: agy --dangerously-skip-permissions --print {prompt}
agents:
  default: agy
state_agents:
  ai-review: gemini
```

## Usage

Start the orchestrator:

```bash
export GITHUB_TOKEN=<YOUR_GITHUB_TOKEN>
# Example: Managing issues for 'my-org/my-project'
orchestrator-loop-for-ai --repo my-project
```

### Options

- `--once`: Runs the orchestrator loop only once and exits.
- `--status`: Checks the status of the next actionable issue, outputs the command it would run, and exits.

The orchestrator will poll for issues assigned to you with the `ai-enabled` label and begin the SDLC process.

## Local Workspace

Workspaces for each issue are created in `~/ai-workspaces/<repo_name>-<issue_number>`.

## License

MIT
