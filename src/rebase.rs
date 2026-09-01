use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::config::AutoRebaseConfig;
use crate::github::GithubClient;
use crate::workspace::Workspace;

/// Records, per issue workspace, the `origin/<main>` sha we last reacted to.
pub const REBASE_STATE_FILE: &str = ".rebase_state.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RebaseOutcome {
    /// Nothing to do: no checkout, no open PR, or `origin/main` has not moved.
    Skipped,
    /// `origin/main` moved but the branch already contained it.
    AlreadyCurrent,
    /// The branch was replayed onto the new `origin/main` and force-pushed.
    Rebased,
    /// The rebase hit conflicts and was aborted; the PR needs a human.
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseState {
    pub last_main_sha: String,
    pub outcome: RebaseOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchSync {
    NoRemote,
    InSync,
    LocalAhead,
    LocalBehind,
    Diverged,
}

#[derive(Debug, Clone)]
pub struct RebaseResult {
    pub head_before: String,
    pub head_after: String,
    pub conflicted: bool,
}

pub fn read_rebase_state(issue_dir: &Path) -> Option<RebaseState> {
    let content = std::fs::read_to_string(issue_dir.join(REBASE_STATE_FILE)).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn write_rebase_state(issue_dir: &Path, state: &RebaseState) -> Result<()> {
    std::fs::write(
        issue_dir.join(REBASE_STATE_FILE),
        serde_json::to_string_pretty(state)?,
    )?;
    Ok(())
}

/// A sha is "new" until we have recorded an attempt against it, so a conflicted
/// rebase is retried only once main actually moves again.
pub fn is_new_main_sha(state: Option<&RebaseState>, main_sha: &str) -> bool {
    state.map(|s| s.last_main_sha != main_sha).unwrap_or(true)
}

pub fn parse_pr_number(gh_json: &str) -> Result<Option<u64>> {
    let trimmed = gh_json.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let prs: Vec<serde_json::Value> = serde_json::from_str(trimmed)?;
    Ok(prs.first().and_then(|pr| pr["number"].as_u64()))
}

fn git(repo_dir: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .current_dir(repo_dir)
        .args(args)
        .output()
        .map_err(|e| anyhow!("Failed to run `git {}`: {}", args.join(" "), e))
}

fn git_checked(repo_dir: &Path, args: &[&str]) -> Result<String> {
    let output = git(repo_dir, args)?;
    if !output.status.success() {
        return Err(anyhow!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn fetch_origin(repo_dir: &Path) -> Result<()> {
    git_checked(repo_dir, &["fetch", "--prune", "origin"])?;
    Ok(())
}

pub fn rev_parse(repo_dir: &Path, rev: &str) -> Result<String> {
    git_checked(repo_dir, &["rev-parse", rev])
}

pub fn working_tree_clean(repo_dir: &Path) -> Result<bool> {
    Ok(git_checked(repo_dir, &["status", "--porcelain"])?.is_empty())
}

pub fn branch_exists_locally(repo_dir: &Path, branch: &str) -> Result<bool> {
    let output = git(repo_dir, &["rev-parse", "--verify", &format!("refs/heads/{}", branch)])?;
    Ok(output.status.success())
}

pub fn rebase_in_progress(repo_dir: &Path) -> bool {
    repo_dir.join(".git/rebase-merge").exists() || repo_dir.join(".git/rebase-apply").exists()
}

/// Leaves behind any half-finished rebase or merge from an earlier crashed run.
pub fn abort_in_progress_operations(repo_dir: &Path) -> Result<()> {
    if rebase_in_progress(repo_dir) {
        println!("  {} Aborting an in-progress rebase before syncing.", "WARN:".yellow());
        git(repo_dir, &["rebase", "--abort"])?;
    }
    if repo_dir.join(".git/MERGE_HEAD").exists() {
        println!("  {} Aborting an in-progress merge before syncing.", "WARN:".yellow());
        git(repo_dir, &["merge", "--abort"])?;
    }
    Ok(())
}

/// Compares the local branch with `origin/<branch>` so we never force-push over
/// commits we do not have.
pub fn branch_sync_status(repo_dir: &Path, branch: &str) -> Result<BranchSync> {
    let remote_ref = format!("refs/remotes/origin/{}", branch);
    if !git(repo_dir, &["rev-parse", "--verify", &remote_ref])?.status.success() {
        return Ok(BranchSync::NoRemote);
    }

    let local = rev_parse(repo_dir, &format!("refs/heads/{}", branch))?;
    let remote = rev_parse(repo_dir, &remote_ref)?;
    if local == remote {
        return Ok(BranchSync::InSync);
    }

    let remote_in_local = git(repo_dir, &["merge-base", "--is-ancestor", &remote, &local])?
        .status
        .success();
    let local_in_remote = git(repo_dir, &["merge-base", "--is-ancestor", &local, &remote])?
        .status
        .success();

    Ok(match (remote_in_local, local_in_remote) {
        (true, false) => BranchSync::LocalAhead,
        (false, true) => BranchSync::LocalBehind,
        _ => BranchSync::Diverged,
    })
}

/// Replays `branch` onto `onto`, aborting (and reporting) instead of leaving
/// conflict markers behind.
pub fn rebase_branch_onto(repo_dir: &Path, branch: &str, onto: &str) -> Result<RebaseResult> {
    git_checked(repo_dir, &["checkout", branch])?;
    let head_before = rev_parse(repo_dir, "HEAD")?;

    let output = git(repo_dir, &["rebase", onto])?;
    if !output.status.success() {
        git(repo_dir, &["rebase", "--abort"])?;
        return Ok(RebaseResult {
            head_after: head_before.clone(),
            head_before,
            conflicted: true,
        });
    }

    let head_after = rev_parse(repo_dir, "HEAD")?;
    Ok(RebaseResult { head_before, head_after, conflicted: false })
}

pub fn force_push(repo_dir: &Path, branch: &str) -> Result<()> {
    git_checked(repo_dir, &["push", "--force-with-lease", "origin", branch])?;
    Ok(())
}

/// Runs the repo's publish script. Returns false when the repo has no such script.
pub fn run_publish_script(repo_dir: &Path, script: &str) -> Result<bool> {
    let script_path = repo_dir.join(script);
    if !script_path.is_file() {
        return Ok(false);
    }

    println!("  {} Running {} in {:?}", "INFO:".blue(), script, repo_dir);
    let status = Command::new(&script_path)
        .current_dir(repo_dir)
        .status()
        .map_err(|e| anyhow!("Failed to run {}: {}", script, e))?;

    if !status.success() {
        return Err(anyhow!("{} exited with {}", script, status));
    }
    Ok(true)
}

/// Watches `origin/<main>` for each issue workspace and keeps that issue's PR
/// branch rebased on top of it, republishing the build when the rebase lands.
pub struct AutoRebaser<'a> {
    config: &'a AutoRebaseConfig,
    workspace: &'a Workspace,
}

impl<'a> AutoRebaser<'a> {
    pub fn new(config: &'a AutoRebaseConfig, workspace: &'a Workspace) -> Self {
        Self { config, workspace }
    }

    pub fn branch_name(issue_number: u64) -> String {
        format!("issue-{}", issue_number)
    }

    /// Everything happens inside `~/ai-workspaces/<repo>-<issue>/<repo>`.
    pub async fn process_issue(
        &self,
        github: &GithubClient,
        repo: &str,
        issue_number: u64,
    ) -> Result<RebaseOutcome> {
        let issue_dir = self.workspace.get_issue_dir(repo, issue_number);
        let repo_dir = issue_dir.join(repo);
        let branch = Self::branch_name(issue_number);

        if !repo_dir.join(".git").exists() || !branch_exists_locally(&repo_dir, &branch)? {
            return Ok(RebaseOutcome::Skipped);
        }

        fetch_origin(&repo_dir)?;
        let main_ref = format!("origin/{}", self.config.main_branch);
        let main_sha = rev_parse(&repo_dir, &main_ref)?;

        let state = read_rebase_state(&issue_dir);
        if !is_new_main_sha(state.as_ref(), &main_sha) {
            return Ok(RebaseOutcome::Skipped);
        }

        let Some(pr_number) = self.open_pr_number(&repo_dir, &branch)? else {
            return Ok(RebaseOutcome::Skipped);
        };

        println!(
            "{} {} moved to {} - rebasing PR #{} for issue #{}",
            "INFO:".blue(),
            main_ref,
            &main_sha[..7.min(main_sha.len())],
            pr_number,
            issue_number
        );

        abort_in_progress_operations(&repo_dir)?;

        if !working_tree_clean(&repo_dir)? {
            println!(
                "  {} Uncommitted changes in {:?}; leaving the rebase for later.",
                "WARN:".yellow(),
                repo_dir
            );
            return Ok(RebaseOutcome::Skipped);
        }

        match branch_sync_status(&repo_dir, &branch)? {
            BranchSync::LocalBehind => {
                println!("  {} Fast-forwarding {} to origin/{}", "INFO:".blue(), branch, branch);
                git_checked(&repo_dir, &["checkout", &branch])?;
                git_checked(&repo_dir, &["reset", "--hard", &format!("origin/{}", branch)])?;
            }
            BranchSync::Diverged => {
                println!(
                    "  {} {} has diverged from origin/{}; skipping to avoid discarding commits.",
                    "WARN:".yellow(),
                    branch,
                    branch
                );
                return Ok(RebaseOutcome::Skipped);
            }
            BranchSync::NoRemote | BranchSync::InSync | BranchSync::LocalAhead => {}
        }

        let result = rebase_branch_onto(&repo_dir, &branch, &main_ref)?;

        if result.conflicted {
            self.record(&issue_dir, &main_sha, RebaseOutcome::Conflict)?;
            println!(
                "  {} Rebase of {} onto {} conflicts; aborted and left for a human.",
                "WARN:".yellow(),
                branch,
                main_ref
            );
            github
                .post_comment(
                    issue_number,
                    &format!(
                        "🔀 `{}` moved to `{}`, but automatically rebasing `{}` hit merge conflicts, \
                         so the rebase was aborted and PR #{} was left untouched. \
                         It needs a manual rebase before the dev build can be refreshed.",
                        main_ref,
                        &main_sha[..7.min(main_sha.len())],
                        branch,
                        pr_number
                    ),
                )
                .await?;
            return Ok(RebaseOutcome::Conflict);
        }

        if result.head_after == result.head_before {
            self.record(&issue_dir, &main_sha, RebaseOutcome::AlreadyCurrent)?;
            println!("  {} {} already contains {}; nothing to publish.", "INFO:".blue(), branch, main_ref);
            return Ok(RebaseOutcome::AlreadyCurrent);
        }

        force_push(&repo_dir, &branch)?;
        // Recorded before publishing so a build failure never re-triggers the rebase.
        self.record(&issue_dir, &main_sha, RebaseOutcome::Rebased)?;
        println!("  {} Rebased and force-pushed {} (PR #{})", "INFO:".blue(), branch, pr_number);

        match run_publish_script(&repo_dir, &self.config.publish_script) {
            Ok(true) => println!("  {} Published a fresh build for {}", "INFO:".blue(), branch),
            Ok(false) => println!(
                "  {} No {} in {}; skipping publish.",
                "INFO:".blue(),
                self.config.publish_script,
                repo
            ),
            Err(e) => {
                eprintln!("  {} Publish failed after rebase: {}", "ERROR:".red(), e);
                github
                    .post_comment(
                        issue_number,
                        &format!(
                            "✅ `{}` was rebased onto `{}` (`{}`), but `{}` failed: {}",
                            branch,
                            main_ref,
                            &main_sha[..7.min(main_sha.len())],
                            self.config.publish_script,
                            e
                        ),
                    )
                    .await?;
            }
        }

        Ok(RebaseOutcome::Rebased)
    }

    fn record(&self, issue_dir: &Path, main_sha: &str, outcome: RebaseOutcome) -> Result<()> {
        write_rebase_state(
            issue_dir,
            &RebaseState { last_main_sha: main_sha.to_string(), outcome },
        )
    }

    fn open_pr_number(&self, repo_dir: &PathBuf, branch: &str) -> Result<Option<u64>> {
        let output = Command::new("gh")
            .current_dir(repo_dir)
            .args(["pr", "list", "--head", branch, "--state", "open", "--json", "number"])
            .output()
            .map_err(|e| anyhow!("Failed to run `gh pr list`: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "`gh pr list` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        parse_pr_number(&String::from_utf8_lossy(&output.stdout))
    }
}

#[cfg(test)]
#[path = "rebase_tests.rs"]
mod rebase_tests;
