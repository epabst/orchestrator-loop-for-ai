# Technical Design Document: Orchestrator Loop for AI

## 1. Introduction
The Orchestrator Loop for AI is an autonomous CLI system designed to streamline software development by automating task management and agent delegation based on the software development lifecycle (SDLC).

## 2. System Architecture
The system employs a modular, event-driven architecture centered around a state machine that orchestrates tasks through predefined SDLC phases.

### 2.1 Core Components
*   **WorkPoller:** Responsible for interfacing with external providers (e.g., GitHub) to fetch tasks/issues.
*   **TaskOrchestrator:** The central brain managing the state machine. It consumes tasks, determines the current state, and triggers appropriate transitions.
*   **AgentManager:** Maintains the registry of specialized AI agents and executes delegated tasks based on the current SDLC phase.
*   **StateEngine:** Manages the persistence of state transitions using a YAML-driven configuration.
*   **VisualizationModule:** A background process/hook that monitors changes in the state machine configuration and updates the `README.md` file with an up-to-date Mermaid state diagram.

## 3. State Machine Design
The orchestrator operates as a deterministic state machine. State transition rules are defined in a central `state-machine.yaml` to decouple logic from the implementation.

### 3.1 States
*   `ai-requirements`
*   `ai-design`
*   `ai-development`
*   `ai-review`
*   `ai-pr-creation`
*   `ai-done`
*   `ai-human-help` (Exception/Escalation path)

### 3.2 YAML Configuration Schema
```yaml
transitions:
  - from: ai-requirements
    to: ai-design
    agent_task: generate_spec
  - from: ai-design
    to: ai-development
    agent_task: implement_solution
  # ... etc
```

## 4. Data Persistence
State persistence for active tasks will be managed locally to ensure robustness:
*   **Task Repository:** A local SQLite database or structured JSON files to store active task metadata, current state, and history.

## 5. Workflow Sequence
1.  **Poll:** `WorkPoller` fetches new/updated issues.
2.  **Initialize:** `TaskOrchestrator` creates a new state record for the task.
3.  **Process:** The orchestrator identifies the current state, fetches the action from the configuration, and triggers `AgentManager`.
4.  **Execute:** `AgentManager` interacts with the AI service.
5.  **Transition:** Upon success, `TaskOrchestrator` updates the state; upon failure, it triggers `ai-human-help`.
6.  **Visualize:** `VisualizationModule` updates `README.md`.

## 6. Implementation Strategy
*   **Language:** Python or Go (CLI optimization).
*   **Modular Design:** Clearly defined interfaces for `Poller`, `Agent`, and `Persistence` to allow future support for alternative sources (e.g., GitLab, Jira) or agents.
*   **Validation:** Every state transition must be validated against the YAML configuration before execution.

COMPLETED_DESIGN
