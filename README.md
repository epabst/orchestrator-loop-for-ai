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
- **Auto-Rebase & Publish:** Every open PR for an `ai-enabled` issue gets a dev build for its current head — published when the PR is created, and again whenever `origin/main` moves and the branch is rebased in its workspace. The issue then waits in `ai-awaiting-pr-merge`, open or closed, until the PR merges; the workspace switches to `main`, publishes a `*-main.apk` so it is obvious which build to install, and only then is the issue `ai-done`.
- **Human-in-the-loop:** Easily pause for human help by adding an `ai-human-help` label.
- **Configurable Agents:** Map different SDLC states to different AI agents (defaulting to `agy`).
- **Flexible Prompt Delivery:** Supports both stdin-based agents (e.g., `gemini`) and CLI-argument-based agents (e.g., `agy`) via the `{prompt}` placeholder.
- **Live Agent Transcript:** Shows the agent's tool calls, their results, and its prose as they happen, instead of one silent wait followed by a wall of text.
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
    
    ai-pr-creation --> ai-awaiting-pr-merge
    ai-pr-creation --> ai-human-help
    
    ai-awaiting-pr-merge --> ai-done
    
    ai-done --> [*]
```

Each state represents a phase in the software development lifecycle. On successful task execution, the orchestrator transitions to the next state. On failure, the task escalates to `ai-human-help` for manual intervention.

`ai-awaiting-pr-merge` is the one state no agent runs: the orchestrator handles it
itself (`native_handler: await-pr-merge`). While an issue sits there its PR keeps
being rebased onto `origin/main` and rebuilt, and once the PR is merged the
workspace is checked out on `main`, a `*-main` build is published, and only then
does the issue move to `ai-done`. The orchestrator polls this one state whether the
issue is open **or closed**, so closing an issue the moment you merge its PR - the
usual reflex - still leaves the merge build to be published rather than skipping it.

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
  - The agent runs unattended, so it must be granted tool permissions up front — see [Agent permissions](#agent-permissions).

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

Optional keys `stream_json` and `partial_messages` control the live transcript;
see [Watching the agent work](#watching-the-agent-work).

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

#### Seeing which phase is running

Each SDLC step opens with the phase name as word art, so it is impossible to
miss when scrolling back through a long run, followed by the agent handling that
phase and the exact command line the orchestrator invokes:

```
   ██  ███     ███  ████ █   █ ████ █     ██  ███  █   █ ████ █   █ ███
  █  █  █      █  █ █    █   █ █    █    █  █ █  █ ██ ██ █    ██  █  █
  ████  █  ███ █  █ ███  █   █ ███  █    █  █ ███  █ █ █ ███  █ █ █  █
  █  █  █      █  █ █     █ █  █    █    █  █ █    █   █ █    █  ██  █
  █  █ ███     ███  ████   █   ████ ████  ██  █    █   █ ████ █   █  █

  STATE: ai-development · agent: sdlc-dev · claude --output-format stream-json --verbose --print --agent sdlc-dev {prompt}
```

`orchestrator-loop-for-ai --status` prints the same block for the issue it would
pick up next, which is the quickest way to confirm a `state_agents` override is
wired up the way you expect.

#### Watching the agent work

Headless agents normally print nothing until they exit, which makes a long
`ai-development` step indistinguishable from a hung one. For Claude, the
orchestrator asks for streaming output and renders it as the run happens:

```
* agent session started (claude-opus-5, cwd /Users/you/ai-workspaces/myrepo-42)
> Read(src/parser.rs)
  <- pub fn parse(input: &str) -> Result<Ast> {
     ... 118 more lines
> Edit(src/parser.rs)
  <- The file src/parser.rs has been updated.
> Bash(cargo test)
  <- test result: ok. 41 passed; 0 failed
Added the trailing-comma case and its test.
COMPLETED_DEVELOPMENT
* agent run finished (94.2s, 12 turns, $0.4130)
```

This is on by default. When `command_template` invokes `claude` in print mode,
the orchestrator inserts `--output-format stream-json --verbose` right after the
binary, so a template like

```yaml
command_template: claude --print --agent {agent} {prompt}
```

actually runs as `claude --output-format stream-json --verbose --print --agent
sdlc-dev <prompt>`. Two optional settings adjust it:

```yaml
stream_json: true        # default; set to false to keep the agent's raw output
partial_messages: false  # set to true to stream prose token by token
```

The flags are only added for `claude ... --print` templates that do not already
set `--output-format`. Other agents are launched exactly as written, and their
plain-text output still streams through line by line.

The same text — without the terminal colors — is what lands in the audit log at
`~/ai-workspaces/audit/<repo>/<issue>/<state>_<timestamp>.log`. Only the agent's
prose is scanned for the state's completion keyword, so a keyword that happens
to appear inside a tool call or a file being edited will not advance the state
machine.

#### Agent permissions

The orchestrator invokes the agent non-interactively, so **there is no human available to approve tool prompts**. Any tool call that would normally ask for approval is denied instead, and the agent reports the task as blocked rather than doing the work. The failure shows up in the audit log (`~/ai-workspaces/audit/<repo>/<issue>/`) and looks like this:

```
I could not complete the implementation — all file writes and build/test
commands are blocked in this session, so nothing was changed on disk.
```

This is easy to misread as a code or sync problem: the review state sees an unchanged working tree and reports `NEED_HUMAN_HELP`, which parks the issue behind the `ai-human-help` label. Grant permissions up front so it cannot happen.

At minimum the agent needs:

| Capability | Why the SDLC states need it |
| --- | --- |
| Read, write, edit, glob, grep | `ai-development` implements; every state reads the repo |
| `git` | branch, commit, and push the issue branch |
| `gh` | read the issue, create and update the PR in `ai-pr-creation` |
| Build and test toolchain | `ai-development` is required to verify its own work |

Deny the rest — `sudo`, force-pushes to `main`, repository deletion, and reads of `.env`, `~/.ssh` and `~/.aws`. A broad allow-list beats blanket bypass here, because the agent is acting on real branches unattended.

**Claude.** Permissions come from a settings file, and behavior from an agent definition:

1. `~/.claude/agents/sdlc-dev.md` — the agent's operating instructions: never stop to ask for approval, `cd` into the repository subdirectory, verify by actually building and testing, and emit the state's completion keyword exactly so the state machine can advance. It also pins the test command: run only the project's fastest full-signal task — for Kotlin Compose Multiplatform repos that is `./gradlew desktopTest`, never the whole `test`/`check`/`allTests` matrix — and add another platform's test task only when the change is specific to that platform.
2. `~/ai-workspaces/sdlc-agent-settings.json` — the permission grants:

```json
{
  "permissions": {
    "defaultMode": "acceptEdits",
    "allow": [
      "Read", "Write", "Edit", "Glob", "Grep",
      "Bash(git:*)", "Bash(gh:*)",
      "Bash(./gradlew:*)", "Bash(cargo:*)", "Bash(npm:*)", "Bash(python3:*)"
    ],
    "deny": [
      "Bash(sudo:*)",
      "Bash(git push --force origin main:*)",
      "Bash(gh repo delete:*)",
      "Read(./.env)",
      "Read(~/.ssh/**)"
    ]
  }
}
```

3. Wire both into `command_template`:

```yaml
command_template: claude --print --agent {agent} --settings /Users/you/ai-workspaces/sdlc-agent-settings.json --permission-mode acceptEdits {prompt}
agents:
  default: sdlc-dev
state_agents: {}
```

`--permission-mode acceptEdits` covers file writes; the `allow` list covers shell commands. Both are needed — `acceptEdits` alone still blocks `Bash`.

**Command form matters as much as the allow-list.** Two of Claude Code's own
rules sit in front of the grants above, and an agent that trips them sees
"requires approval", not a shell error:

| Form | Result |
| --- | --- |
| `cd <repo> && ./gradlew desktopTest` | allowed — relative path, inside the workspace |
| `cd /guessed/path && ./gradlew desktopTest` | refused — `cd` outside the working directory |
| `<repo>/gradlew -p <repo> desktopTest` | refused — the grant is `Bash(./gradlew:*)`, not a path-qualified wrapper |
| `git -C <repo> log` | allowed |
| `cd <repo> && git log` | refused — a directory change before `git` can run untrusted hooks |

So the orchestrator's prompt names the workspace and the checkout by absolute
path, and `sdlc-dev.md` tells the agent to use those paths verbatim and to reach
the build wrapper with `cd <repo> && ./gradlew`. Without that, an agent that
invents a path from the branch name (`~/ai-workspaces/issue-54/<repo>`) has its
verification silently refused and reports the work as untested.

> **Note:** a `permissions:` block inside the agent's own frontmatter is **silently ignored** (verified on Claude Code 2.1.252 — a frontmatter `deny` of `Bash(git:*)` failed to block `git`). Permissions must be supplied via `--settings`, `--permission-mode`, or `--allowedTools`. Frontmatter is for behavior only.

**Other agents.** `agy` takes `--dangerously-skip-permissions`, as shown above. For `gemini`, the orchestrator sets `GEMINI_CLI_TRUST_WORKSPACE=true` automatically on every invocation, so no extra configuration is needed.

Whatever agent you use, verify the grants before starting a real run — a five-second check beats a state machine that silently makes no progress:

```bash
cd ~/ai-workspaces && <your command_template, with a test prompt> \
  'Report ALLOWED or BLOCKED for each: (1) write a file proof.txt, (2) run `git status`, (3) run `gh --version`, (4) run `sudo -n true`.'
```

Expect the first three to be allowed and `sudo` to be blocked. If a write is blocked, the permission wiring is not in effect.

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

1. `git fetch origin`, then skip the issue unless it has an **open PR** for its
   `issue-<number>` branch.
2. Compare `origin/main` against the sha last handled for that issue (recorded in
   `~/ai-workspaces/<repo>-<issue>/.rebase_state.json`). If it has not moved,
   there is nothing to rebase — but the publish check in step 4 still runs.
3. Rebase `issue-<number>` onto `origin/main`.
   - **On conflict:** the rebase is left in progress and handed to the same
     agent that resolves conflicts during `ai-pr-creation` (an audit log is
     written to `~/ai-workspaces/audit/<repo>/<issue>/`). If the agent resolves
     the conflicts and finishes the rebase, it proceeds as a success below. If
     it fails, the rebase is aborted, the branch is left exactly as it was, and
     a comment is posted on the issue asking for a manual rebase. Either way,
     the same conflict is not retried until `origin/main` moves again.
   - **On success:** the branch is force-pushed (`--force-with-lease`), which
     updates the PR.
4. Publish a build for the branch head, unless that exact head already has one.
   The repo's `publish-android-dev.sh` is run from the repo root, building the new
   `.apk` and copying it to Google Drive; the head is then recorded as published in
   `.rebase_state.json`. Repos without that script simply skip this step, and a
   failed build leaves the head unpublished so the next pass retries it.

Publishing keys off the **branch head**, not off the rebase, so every head gets
exactly one build. That matters most at the end of the SDLC: `ai-pr-creation`
rebases onto `origin/main` before it pushes, so a freshly opened PR is already
current and no rebase will ever move it. The same publish step therefore runs
right after the PR is created or updated, and a PR is never left without a build
a reviewer can install.

When a rebase happens, the console shows it end to end:

```
REBASE: origin/main moved to 07d4f07 - PR #99 (issue-7) needs a rebase
  PR: https://github.com/owner/demo/pull/99
  INFO: Rebasing issue-7 onto origin/main...
  OK: Rebased 1 commit(s) onto 07d4f07 and force-pushed - PR #99 is up to date (380441b -> da9481f)
  INFO: Running publish-android-dev.sh to publish a build for issue-7 (da9481f)...
  OK: Published a new build for issue-7
REBASE: Pass complete: 1 rebased, 0 merged, 0 conflicted, 0 errored.
```

A conflict reports itself just as loudly, and a pass where every PR is already
current stays silent.

### After the PR is merged

Step 1 finding no open PR is not always the end: if `gh` reports a **merged** PR
for `issue-<number>`, that issue's work now lives in `main`, and the build worth
installing is main's. So the workspace is switched over:

1. Abort any half-finished rebase or merge, and skip the issue while the working
   tree is dirty.
2. `git checkout -B main origin/main`, which parks the checkout on the freshly
   merged `main`. A local `main` holding commits the origin does not have is left
   alone instead of being reset.
3. Run the publish script from there, producing a `*-main.apk` rather than a
   branch build.

The merged PR number is recorded in `.rebase_state.json`, so this happens **once
per merge**, not on every pass. With that recorded, the issue leaves
`ai-awaiting-pr-merge` for `ai-done` — the merge is only finished once the build
exists.

The ordinary issue poll asks GitHub for *open* issues only, which used to mean a
merge went unpublished whenever the issue was closed before the next pass — the
common case, since closing an issue is the natural thing to do the moment you
merge its PR. So issues labelled `ai-awaiting-pr-merge` are fetched by a second,
label-scoped query that ignores open/closed, and stay visible to the rebase pass
until their main build is published.

```
REBASE: PR #99 (issue-7) was merged - moving the checkout to main (07d4f07) for a fresh build
  PR: https://github.com/owner/demo/pull/99
  OK: Checked out main at 07d4f07
  INFO: Running publish-android-dev.sh to publish a build for main (07d4f07)...
  OK: Published a new build for main
  OK: PR #99 is merged and published - issue #7 moves to ai-done
REBASE: Pass complete: 0 rebased, 1 merged, 0 conflicted, 0 errored.
```

Safety guards: issues currently being processed (holding the workspace lock or
carrying the `ai-processing` label) are left alone, workspaces with uncommitted
changes are skipped, and a branch that has diverged from its remote is skipped
rather than force-pushed over.

Configure it in `sdlc_config.yaml`:

```yaml
auto_rebase:
  enabled: true                          # set false to turn the feature off
  main_branch: main                      # base branch to track
  publish_script: publish-android-dev.sh # run from the repo root, once per branch head
  check_interval_secs: 300               # how often to fetch and compare
```

## Local Workspace

Workspaces for each issue are created in `~/ai-workspaces/<repo_name>-<issue_number>`.

## License

MIT
