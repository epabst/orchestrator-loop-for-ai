# Contributing (project constitution)

Steering rules for this project that outlive any single issue. The
ai-development and ai-review steps read this file and keep it current;
changes arrive through the PR like any other change.

## Process
- New issues go straight to `ai-development`; requirements and design are skipped.
- Every decision is a bullet. ai-development reports "Existing decisions" (still valid) and "New decisions".
- The issue comment lists only new decisions; the PR description lists all, new ones marked.
- ai-review labels concerns by level with a code: Blocker (B1), Major (M1), Minor (m1), Nitpicks (N1). Only Blocker/Major fail a review.

## Code
- Write a failing test first, then the implementation.
- Tests: `cargo test`.
- Keep agent-facing conventions in `sdlc_config.yaml` prompts; keep Rust changes small.
