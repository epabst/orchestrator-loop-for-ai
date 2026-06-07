# Technical Design Document: Orchestrator Loop for AI

## 1. Overview
The "Orchestrator Loop for AI" is a CLI-based system designed to automate the SDLC by monitoring external sources (e.g., GitHub issues) and orchestrating AI agents through a defined state machine.

## 2. System Architecture

### 2.1 Core Components
- **Orchestrator Loop:** The central engine that manages the polling, task parsing, state transitions, and agent delegation.
- **Polling Module:** Responsible for periodic fetching of tasks from configured sources.
- **State Machine Engine:** Manages persistence and transitions based on `sdlc_config.yaml`.
- **Agent Manager:** Interfaces with LLM APIs to delegate tasks based on current SDLC state.
- **UI Module:** Manages console output, including the "waiting" status optimization.
- **Visualization Module:** Generates and updates Mermaid diagrams in `README.md`.

### 2.2 Data Flow
1. **Polling:** The system fetches issues/tasks from GitHub via the `github.rs` module.
2. **Orchestration:** The main loop processes each task through the state machine.
3. **Execution:** The `agent.rs` module invokes the appropriate AI agent for the current state.
4. **Transition:** Upon agent completion, the state is updated, persisted, and the transition diagram is regenerated in `README.md`.

## 3. Detailed Component Design

### 3.1 State Management (`sdlc.rs`, `sdlc_config.yaml`)
The state machine will be driven by the configuration in `sdlc_config.yaml`. The `sdlc.rs` module will load this configuration and maintain the current state for each task in a local state store (e.g., file-based or database-backed).

### 3.2 Polling & Console Output (`main.rs`)
To satisfy requirement 5.1 (Console Output Optimization), the main loop will utilize terminal escape sequences (e.g., `\r` for carriage return) to overwrite the current line in the console during the "waiting" state, rather than printing new lines.

### 3.3 Visualization (`README.md`)
The `Visualization Module` will parse `sdlc_config.yaml` and update the Mermaid diagram block in `README.md` whenever the configuration is modified or at initialization to ensure consistency.

## 4. Configuration Schema (`sdlc_config.yaml`)
The schema will define:
- Polling interval.
- Source configuration (e.g., repository URL).
- States and valid transitions.
- Agent mappings for each state.

## 5. Security & Maintenance
- **Secrets:** All API keys must be handled via environment variables (e.g., `GITHUB_TOKEN`, `AI_API_KEY`).
- **Extensibility:** The modular design ensures new agents or states can be added by updating `sdlc_config.yaml` and implementing the corresponding logic in the `Agent Manager`.

COMPLETED_DESIGN
