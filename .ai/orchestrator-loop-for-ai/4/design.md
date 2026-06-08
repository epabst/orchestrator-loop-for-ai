# Technical Design Document: SDLC Document Management Refactoring

## 1. Overview
The goal of this project is to modify the `orchestrator-loop-for-ai` CLI tool to cease the automatic synchronization of AI-generated documentation (requirements and design specifications) to the underlying GitHub repository. This will eliminate unnecessary commits and clutter within the repository's `.ai/` directory.

## 2. System Architecture Changes
The current implementation utilizes an SDLC loop, likely managed in `src/sdlc.rs`, which triggers document synchronization after generation or updates. The proposed change involves decoupling document lifecycle management from repository synchronization.

### 2.1 Proposed Workflow
1.  **Document Generation/Editing:** The agent continues to generate/update `requirements.md` and `design.md` as part of the normal operation.
2.  **Storage:** These files will be stored exclusively in the local working directory defined for the current session/task.
3.  **Repository Sync:** The mechanism that pushes these files to the remote repository (via GitHub API) will be removed or deactivated.

## 3. Implementation Plan

### 3.1 Code Changes
*   **`src/sdlc.rs`:**
    *   Identify the entry point for document synchronization (likely named `sync_documents` or similar).
    *   Remove or comment out the calls to this function in the main SDLC loop.
    *   Ensure that the removal of this call does not cause side effects in the control flow.
*   **`src/workspace.rs`:**
    *   Verify how the workspace manager interacts with the `.ai/` directory.
    *   Confirm that file reads/writes for requirements and designs still point to the correct local workspace path.
*   **`sdlc_config.yaml`:**
    *   Review for any flags or configurations that enable/disable repository synchronization. Remove these if they are no longer needed.

### 3.2 Testing Strategy
*   **Unit Tests:** Add or update tests in `src/workspace_tests.rs` and `src/config_tests.rs` to ensure that files are created in the local directory without attempting to sync to any remote path.
*   **Integration Tests:** Execute the SDLC loop and verify that no Git commits are triggered by the tool itself for documentation changes.
*   **Verification:** Run the tool locally, generate documents, and verify the file system state to ensure files exist in the workspace, but do *not* exist in the repository's `.ai/` directory.

## 4. Risks and Mitigation
*   **Risk:** Document availability for future steps or for the user.
    *   **Mitigation:** Confirm that the local workspace persists beyond the tool's execution and is accessible to the user and subsequent agent sessions.
*   **Risk:** Code regressions due to removing core functionality.
    *   **Mitigation:** Thorough testing of the SDLC loop logic after removing the synchronization call.

COMPLETED_DESIGN
