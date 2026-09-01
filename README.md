# Orchestrator Loop for AI

A Rust-based CLI tool that manages an AI-driven software development lifecycle (SDLC) using GitHub issues as the state controller.

This is a script that runs in a loop to poll for work to do (e.g., GitHub issues) and invokes AI agents for each phase of the software development lifecycle (SDLC).

## Overview

The Orchestrator Loop for AI automates software development tasks by:
1. **Polling** external sources (e.g., GitHub) for new tasks/issues
2. **Orchestrating** tasks through a deterministic state machine
3. **Delegating** work to specialized AI agents based on the current SDLC phase
4. **Managing state** persistence and transitions

## Features

- **GitHub-First Workflow:** Uses GitHub labels and comments to drive the SDLC.
- **Multi-Repo Support:** Run without `--repo` to automatically discover and process assigned issues across all your repositories.
- **Automated SDLC Stages:**
  - `ai-requirements`: Requirements gathering and spec document creation.
  - `ai-design`: Technical design document creation.
  - `ai-development`: Test-driven development.
  - `ai-review`: Automated code review.
  - `ai-pr-creation`: Automated Pull Request creation.
  - `ai-done`: Completion state.
- **Concurrency Safety:** Uses both GitHub labels (`ai-processing`) and local file locking to ensure only one instance processes an issue at a time.
- **Auto-Rebase & Publish:** When `origin/main` moves, open PRs for `ai-enabled` issues are rebased in their workspace and a fresh build is published.
- **Human-in-the-loop:** Easily pause for human help by adding an `ai-human-help` label.
- **Configurable Agents:** Map different SDLC states to different AI agents (defaulting to `agy`).
- **Flexible Prompt Delivery:** Supports both stdin-based agents (e.g., `gemini`) and CLI-argument-based agents (e.g., `agy`) via the `{prompt}` placeholder.
- **Cross-platform:** Works on Windows, Mac, and Linux.

## System Architecture

### Core Components
- **WorkPoller:** Interfaces with external sources (GitHub, local files) to fetch tasks
- **TaskOrchestrator:** Central coordinator managing state transitions and task lifecycle
- **AgentManager:** Executes delegated tasks using registered AI agents
- **StateEngine:** Persists task state and history using JSON storage
- **VisualizationModule:** Generates and maintains Mermaid state diagrams

### Supported SDLC States
- `ai-requirements` - Initial state for new tasks
- `ai-design` - Design phase
- `ai-development` - Development/implementation phase
- `ai-review` - Code review phase
- `ai-pr-creation` - Pull request creation phase
- `ai-done` - Task completion
- `ai-human-help` - Escalation state for exceptions

## State Transitions

```mermaid
stateDiagram-v2
    [*] --> ai-requirements
    
    ai-requirements --> ai-design
    ai-requirements --> ai-human-help
    
    ai-design --> ai-development
    ai-design --> ai-human-help
    
    ai-development --> ai-review
    ai-development --> ai-human-help
    
    ai-review --> ai-pr-creation
    ai-review --> ai-human-help
    
    ai-pr-creation --> ai-done
    ai-pr-creation --> ai-human-help
    
    ai-done --> [*]
```

Each state represents a phase in the software development lifecycle. On successful task execution, the orchestrator transitions to the next state. On failure, the task escalates to `ai-human-help` for manual intervention.

## Prerequisites

- Rust (installed via [rustup](https://rustup.rs/))
- GITHUB_TOKEN environment variable set with the necessary permissions.
  - **Best Practice**: Use a **Fine-grained PAT**.
  - To generate: Go to [GitHub Settings](https://github.com/settings/tokens) -> "Personal access tokens" -> "Fine-grained tokens" -> "Generate new token".
  - For a **single repository**, select that repo and grant:
    - **Issues**: Read and write
    - **Metadata**: Read-only
    - **Contents**: Read and write
  - For **all-repos mode** (no `--repo` flag), select "All repositories" and grant the same permissions across all repos you want managed.
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

Start the orchestrator in **all-repos mode** (discovers assigned issues across every repository):

```bash
export GITHUB_TOKEN=<YOUR_GITHUB_TOKEN>
orchestrator-loop-for-ai
```

Or target a **specific repository**:

```bash
export GITHUB_TOKEN=<YOUR_GITHUB_TOKEN>
orchestrator-loop-for-ai --repo my-project
# Full URLs also work:
orchestrator-loop-for-ai --repo https://github.com/my-org/my-project
```

### Options

| Flag | Description |
|------|-------------|
| `--repo <name>` | Restrict to a single repository. Omit to process issues across **all** repositories assigned to you. Accepts a repo name or full GitHub URL. |
| `--once` | Run the orchestrator loop only once and exit. |
| `--status` | Print the next actionable issue, its current state, and the command that would run, then exit. |
| `--config <path>` | Path to the SDLC config file (default: `sdlc_config.yaml`). |

The orchestrator will poll every 15 seconds for issues assigned to you with the `ai-enabled` label and begin the SDLC process.

> [!NOTE]
> When using all-repos mode, the GITHUB_TOKEN must have **Issues: Read and write** and **Metadata: Read-only** permissions on every repository you want the orchestrator to manage.

## Auto-Rebase and Publish

Between agent phases, the orchestrator checks whether `origin/main` has moved in
any repository that has an open, `ai-enabled` issue assigned to you. When it has,
each such issue's PR branch is rebased **inside that issue's own checkout** at
`~/ai-workspaces/<repo>-<issue>/<repo>`:

1. `git fetch origin` and compare `origin/main` against the sha last handled for
   that issue (recorded in `~/ai-workspaces/<repo>-<issue>/.rebase_state.json`).
2. Skip the issue unless it has an **open PR** for its `issue-<number>` branch.
3. Rebase `issue-<number>` onto `origin/main`.
   - **On conflict:** the rebase is aborted, the branch is left exactly as it was,
     and a comment is posted on the issue asking for a manual rebase. The same
     conflict is not retried until `origin/main` moves again.
   - **On success:** the branch is force-pushed (`--force-with-lease`), which
     updates the PR.
4. After a successful rebase that actually changed the branch, the repo's
   `publish-android-dev.sh` is run from the repo root, building the new `.apk`
   and copying it to Google Drive. Repos without that script simply skip this step.

Safety guards: issues currently being processed (holding the workspace lock or
carrying the `ai-processing` label) are left alone, workspaces with uncommitted
changes are skipped, and a branch that has diverged from its remote is skipped
rather than force-pushed over.

Configure it in `sdlc_config.yaml`:

```yaml
auto_rebase:
  enabled: true                          # set false to turn the feature off
  main_branch: main                      # base branch to track
  publish_script: publish-android-dev.sh # run from the repo root after a rebase
  check_interval_secs: 300               # how often to fetch and compare
```

## Local Workspace

Workspaces for each issue are created in `~/ai-workspaces/<repo_name>-<issue_number>`.

## License

MIT
