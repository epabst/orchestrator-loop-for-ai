use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::agent::Agent;
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
    /// The rebase hit conflicts and an agent failed to resolve them, so it was
    /// aborted; the PR needs a human.
    Conflict,
    /// The PR was merged, so the checkout was moved to `origin/<main>` and a
    /// build of main was published.
    Merged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebaseState {
    pub last_main_sha: String,
    pub outcome: RebaseOutcome,
    /// Branch head whose build was last published. Publishing keys off the branch
    /// head rather than off `last_main_sha` so a PR gets exactly one build per
    /// head, no matter which code path produced it.
    #[serde(default)]
    pub last_published_sha: Option<String>,
    /// PR whose merge this workspace has already reacted to, so the checkout is
    /// switched to main and a main build published exactly once per merge.
    #[serde(default)]
    pub merged_pr: Option<u64>,
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

/// True until this workspace has reacted to `pr_number` being merged.
pub fn is_new_merged_pr(state: Option<&RebaseState>, pr_number: u64) -> bool {
    state.map(|s| s.merged_pr != Some(pr_number)).unwrap_or(true)
}

/// The merged PR this workspace has already published a build of main for, if
/// any. This is what tells the `await-pr-merge` state its wait is over: the
/// merge happened *and* the build a human should install exists.
pub fn published_merge(issue_dir: &Path) -> Option<u64> {
    read_rebase_state(issue_dir).and_then(|s| s.merged_pr)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPr {
    pub number: u64,
    pub url: String,
}

pub fn parse_open_pr(gh_json: &str) -> Result<Option<OpenPr>> {
    let trimmed = gh_json.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let prs: Vec<serde_json::Value> = serde_json::from_str(trimmed)?;
    Ok(prs.first().and_then(|pr| {
        Some(OpenPr {
            number: pr["number"].as_u64()?,
            url: pr["url"].as_str().unwrap_or_default().to_string(),
        })
    }))
}

/// The PR for `branch` in the given `state` ("open" or "merged"), if any.
/// Free function (rather than an `AutoRebaser` method) so callers that just
/// need a PR lookup - like fetching its review feedback - don't need one.
pub fn pr_for_branch(repo_dir: &Path, branch: &str, state: &str) -> Result<Option<OpenPr>> {
    let output = Command::new("gh")
        .current_dir(repo_dir)
        .args(["pr", "list", "--head", branch, "--state", state, "--json", "number,url"])
        .output()
        .map_err(|e| anyhow!("Failed to run `gh pr list`: {}", e))?;

    if !output.status.success() {
        return Err(anyhow!(
            "`gh pr list` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_open_pr(&String::from_utf8_lossy(&output.stdout))
}

/// One piece of feedback left directly on a pull request: a review verdict
/// (approve / request changes / general comment) or an inline comment on a
/// line of the diff. GitHub models both separately from the issue's comment
/// thread, so `GithubClient::get_issue_comments` never sees them.
#[derive(Debug, Clone, PartialEq)]
pub struct PrFeedback {
    pub author: String,
    pub body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// The file the comment is anchored to, for an inline diff comment.
    pub path: Option<String>,
}

fn parse_pr_feedback(gh_json: &str) -> Result<Vec<PrFeedback>> {
    let trimmed = gh_json.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let items: Vec<serde_json::Value> = serde_json::from_str(trimmed)?;
    Ok(items
        .into_iter()
        .filter_map(|item| {
            let author = item["author"].as_str()?.to_string();
            let body = item["body"].as_str()?.to_string();
            if body.trim().is_empty() {
                // A bare approval with no comment has nothing to show the agent.
                return None;
            }
            let created_at = item["created_at"].as_str()?.parse().ok()?;
            let path = item["path"].as_str().map(|s| s.to_string());
            Some(PrFeedback { author, body, created_at, path })
        })
        .collect())
}

/// Review verdicts (approve / request changes / general comment) left on
/// `pr_number`.
fn pr_reviews(repo_dir: &Path, pr_number: u64) -> Result<Vec<PrFeedback>> {
    let output = Command::new("gh")
        .current_dir(repo_dir)
        .args([
            "api",
            "--paginate",
            &format!("repos/{{owner}}/{{repo}}/pulls/{}/reviews", pr_number),
            "--jq",
            "[.[] | {author: .user.login, body: .body, created_at: .submitted_at}]",
        ])
        .output()
        .map_err(|e| anyhow!("Failed to run `gh api .../reviews`: {}", e))?;
    if !output.status.success() {
        return Err(anyhow!(
            "`gh api .../reviews` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_pr_feedback(&String::from_utf8_lossy(&output.stdout))
}

/// Inline comments left on specific lines of the diff.
fn pr_review_comments(repo_dir: &Path, pr_number: u64) -> Result<Vec<PrFeedback>> {
    let output = Command::new("gh")
        .current_dir(repo_dir)
        .args([
            "api",
            "--paginate",
            &format!("repos/{{owner}}/{{repo}}/pulls/{}/comments", pr_number),
            "--jq",
            "[.[] | {author: .user.login, body: .body, created_at: .created_at, path: .path}]",
        ])
        .output()
        .map_err(|e| anyhow!("Failed to run `gh api .../comments`: {}", e))?;
    if !output.status.success() {
        return Err(anyhow!(
            "`gh api .../comments` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_pr_feedback(&String::from_utf8_lossy(&output.stdout))
}

/// All feedback left directly on `pr_number` - review verdicts and inline
/// diff comments - oldest first. Neither is visible in the issue's own
/// comment thread, which is why the SDLC context file needs this as a
/// separate fetch.
pub fn fetch_pr_feedback(repo_dir: &Path, pr_number: u64) -> Result<Vec<PrFeedback>> {
    let mut feedback = pr_reviews(repo_dir, pr_number)?;
    feedback.extend(pr_review_comments(repo_dir, pr_number)?);
    feedback.sort_by_key(|f| f.created_at);
    Ok(feedback)
}

/// Tag that makes every auto-rebase line easy to spot (and grep for) in the
/// orchestrator's output.
fn tag() -> colored::ColoredString {
    "REBASE:".magenta().bold()
}

fn short_sha(sha: &str) -> &str {
    &sha[..7.min(sha.len())]
}

/// Number of commits the branch carries on top of `main_ref`.
fn commits_ahead(repo_dir: &Path, main_ref: &str) -> Option<String> {
    git_checked(repo_dir, &["rev-list", "--count", &format!("{}..HEAD", main_ref)]).ok()
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

/// Replays `branch` onto `onto`. On conflict the rebase is left mid-flight
/// (conflict markers and all) rather than aborted, so a caller can hand it to
/// an agent to resolve and `git rebase --continue`; callers that give up
/// should call `abort_in_progress_operations` themselves.
pub fn rebase_branch_onto(repo_dir: &Path, branch: &str, onto: &str) -> Result<RebaseResult> {
    git_checked(repo_dir, &["checkout", branch])?;
    let head_before = rev_parse(repo_dir, "HEAD")?;

    let output = git(repo_dir, &["rebase", onto])?;
    if !output.status.success() {
        return Ok(RebaseResult {
            head_after: head_before.clone(),
            head_before,
            conflicted: true,
        });
    }

    let head_after = rev_parse(repo_dir, "HEAD")?;
    Ok(RebaseResult { head_before, head_after, conflicted: false })
}

/// Lists the files a rebase (or merge) has left conflicted in `repo_dir`.
fn conflicted_files(repo_dir: &Path) -> Result<Vec<String>> {
    let output = git_checked(repo_dir, &["diff", "--name-only", "--diff-filter=U"])?;
    Ok(output.lines().map(String::from).collect())
}

/// Hands an in-progress, conflicted rebase to an agent: it resolves the
/// conflicted files, stages them, and runs `git rebase --continue` (repeating
/// if more commits conflict) until the rebase finishes. Returns an error -
/// leaving the rebase mid-conflict for the caller to abort - if the agent
/// doesn't report success or the rebase is still in progress afterward.
pub async fn resolve_conflicts_with_agent(
    repo_dir: &Path,
    issue_dir: &Path,
    audit_log_path: PathBuf,
    merge_conflict_keyword: &str,
    agent_command: String,
) -> Result<()> {
    let mut conflict_prompt = String::from(
        "A rebase conflict has occurred while trying to sync with the latest main branch. \
         Your changes conflicted with recent changes on main. We are mid-rebase (never merge \
         main into this branch - it must stay a clean rebase, with no merge commit).\n\n\
         Please resolve these conflicts:\n\n",
    );
    for file in conflicted_files(repo_dir)? {
        conflict_prompt.push_str(&format!("- {}\n", file));
    }
    conflict_prompt.push_str(&format!(
        "\nInstructions:\n\
         1. Read the conflicted files carefully\n\
         2. Resolve the conflicts by keeping the appropriate code from both versions\n\
         3. Stage all resolved files with 'git add'\n\
         4. Continue the rebase with 'git rebase --continue' (do NOT run 'git merge' \
         or 'git rebase --abort')\n\
         5. If more commits conflict, repeat steps 1-4 until the rebase finishes\n\
         6. Report {} when done\n",
        merge_conflict_keyword
    ));

    let agent = Agent::new(agent_command);
    let response = agent
        .invoke_interactive(&conflict_prompt, audit_log_path, Some(issue_dir.to_path_buf()))
        .await?;

    if !response.contains(merge_conflict_keyword) {
        return Err(anyhow!("Agent failed to resolve rebase conflicts"));
    }
    if rebase_in_progress(repo_dir) {
        return Err(anyhow!(
            "Agent reported rebase conflicts resolved, but a rebase is still in progress in {:?}",
            repo_dir
        ));
    }
    Ok(())
}

pub fn force_push(repo_dir: &Path, branch: &str) -> Result<()> {
    git_checked(repo_dir, &["push", "--force-with-lease", "origin", branch])?;
    Ok(())
}

/// Parks the checkout on `main_branch` at exactly `origin/<main_branch>`, creating
/// the local branch if the clone never had one. Refuses when the local branch
/// carries commits the remote does not, because resetting would throw them away.
pub fn switch_to_main(repo_dir: &Path, main_branch: &str) -> Result<()> {
    if branch_exists_locally(repo_dir, main_branch)? {
        match branch_sync_status(repo_dir, main_branch)? {
            BranchSync::LocalAhead | BranchSync::Diverged => {
                return Err(anyhow!(
                    "local {} has commits origin/{} does not; not resetting it",
                    main_branch,
                    main_branch
                ));
            }
            BranchSync::NoRemote | BranchSync::InSync | BranchSync::LocalBehind => {}
        }
    }
    git_checked(
        repo_dir,
        &["checkout", "-B", main_branch, &format!("origin/{}", main_branch)],
    )?;
    Ok(())
}

/// Runs the repo's publish script. Returns false when the repo has no such script.
pub fn run_publish_script(repo_dir: &Path, script: &str) -> Result<bool> {
    let script_path = repo_dir.join(script);
    if !script_path.is_file() {
        return Ok(false);
    }

    let status = Command::new(&script_path)
        .current_dir(repo_dir)
        .status()
        .map_err(|e| anyhow!("Failed to run {}: {}", script, e))?;

    if !status.success() {
        return Err(anyhow!("{} exited with {}", script, status));
    }
    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    /// The script ran and the branch head was recorded as published.
    Published,
    /// This exact branch head already has a build.
    AlreadyPublished,
    /// The repo has no publish script.
    NoScript,
    /// The checkout is parked on some other branch, so the build would be wrong.
    NotOnBranch,
    /// The script ran and failed; already reported.
    Failed,
}

/// Publishes a build for `branch`'s current head unless that head was already
/// published. This is what makes "an open PR always has a matching dev build"
/// true whether the head came from a rebase or from the SDLC creating the PR.
pub fn publish_if_needed(
    issue_dir: &Path,
    repo_dir: &Path,
    branch: &str,
    script: &str,
) -> Result<PublishOutcome> {
    let head = rev_parse(repo_dir, &format!("refs/heads/{}", branch))?;
    let state = read_rebase_state(issue_dir);
    if state.as_ref().and_then(|s| s.last_published_sha.as_deref()) == Some(head.as_str()) {
        return Ok(PublishOutcome::AlreadyPublished);
    }

    if !repo_dir.join(script).is_file() {
        return Ok(PublishOutcome::NoScript);
    }

    // The script builds whatever is checked out, so refuse to publish from a
    // checkout that is not sitting on the branch we mean to ship.
    if git_checked(repo_dir, &["rev-parse", "--abbrev-ref", "HEAD"])? != branch {
        return Ok(PublishOutcome::NotOnBranch);
    }

    println!(
        "  {} Running {} to publish a build for {} ({})...",
        "INFO:".blue(),
        script,
        branch,
        short_sha(&head)
    );
    run_publish_script(repo_dir, script)?;
    record_published(issue_dir, &head)?;
    Ok(PublishOutcome::Published)
}

/// `publish_if_needed` plus the reporting: a console line, and an issue comment
/// when the build fails. Quiet when there is nothing to do, because this runs on
/// every auto-rebase pass.
pub async fn publish_and_report(
    github: &GithubClient,
    issue_number: u64,
    issue_dir: &Path,
    repo_dir: &Path,
    branch: &str,
    script: &str,
) -> PublishOutcome {
    match publish_if_needed(issue_dir, repo_dir, branch, script) {
        Ok(PublishOutcome::Published) => {
            println!("  {} Published a new build for {}", "OK:".green().bold(), branch);
            PublishOutcome::Published
        }
        Ok(PublishOutcome::NotOnBranch) => {
            println!(
                "  {} {:?} is not checked out on {}; skipping publish.",
                "WARN:".yellow(),
                repo_dir,
                branch
            );
            PublishOutcome::NotOnBranch
        }
        Ok(quiet) => quiet,
        Err(e) => {
            eprintln!("  {} {} failed for {}: {}", "FAILED:".red().bold(), script, branch, e);
            if let Err(comment_err) = github
                .post_comment(
                    issue_number,
                    &format!("⚠️ `{}` failed for `{}`, so the dev build was not refreshed: {}", script, branch, e),
                )
                .await
            {
                eprintln!("  {} Could not comment on the publish failure: {}", "WARN:".yellow(), comment_err);
            }
            PublishOutcome::Failed
        }
    }
}

fn record_published(issue_dir: &Path, head: &str) -> Result<()> {
    let mut state = read_rebase_state(issue_dir).unwrap_or(RebaseState {
        last_main_sha: String::new(),
        outcome: RebaseOutcome::Skipped,
        last_published_sha: None,
        merged_pr: None,
    });
    state.last_published_sha = Some(head.to_string());
    write_rebase_state(issue_dir, &state)
}

/// Remembers that `pr_number`'s merge has been handled, so the main build is
/// published once per merge rather than on every pass.
fn record_merged(issue_dir: &Path, main_sha: &str, pr_number: u64) -> Result<()> {
    let last_published_sha = read_rebase_state(issue_dir).and_then(|s| s.last_published_sha);
    write_rebase_state(
        issue_dir,
        &RebaseState {
            last_main_sha: main_sha.to_string(),
            outcome: RebaseOutcome::Merged,
            last_published_sha,
            merged_pr: Some(pr_number),
        },
    )
}

/// Watches `origin/<main>` for each issue workspace and keeps that issue's PR
/// branch rebased on top of it, republishing the build when the rebase lands.
/// Once the PR is merged it switches that checkout over to `main` and publishes
/// a build from there instead.
pub struct AutoRebaser<'a> {
    config: &'a AutoRebaseConfig,
    workspace: &'a Workspace,
    merge_conflict_keyword: String,
}

impl<'a> AutoRebaser<'a> {
    pub fn new(config: &'a AutoRebaseConfig, workspace: &'a Workspace, merge_conflict_keyword: String) -> Self {
        Self { config, workspace, merge_conflict_keyword }
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

        let Some(pr) = self.open_pr(&repo_dir, &branch)? else {
            // No open PR: either there never was one, or it just got merged and
            // the interesting build is main's rather than the branch's.
            return self
                .publish_main_after_merge(github, issue_number, &issue_dir, &repo_dir, &branch)
                .await;
        };

        let state = read_rebase_state(&issue_dir);
        if !is_new_main_sha(state.as_ref(), &main_sha) {
            // main has not moved, but this head may still be missing a build: the
            // SDLC may have just pushed the PR, or an earlier publish may have failed.
            self.publish(github, issue_number, &issue_dir, &repo_dir, &branch).await;
            return Ok(RebaseOutcome::Skipped);
        }

        println!(
            "{} {} moved to {} - PR #{} ({}) needs a rebase",
            tag(),
            main_ref,
            short_sha(&main_sha).yellow(),
            pr.number,
            branch
        );
        if !pr.url.is_empty() {
            println!("  {} {}", "PR:".blue(), pr.url);
        }

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

        println!("  {} Rebasing {} onto {}...", "INFO:".blue(), branch, main_ref);
        let mut result = rebase_branch_onto(&repo_dir, &branch, &main_ref)?;

        if result.conflicted {
            println!(
                "  {} Merge conflict - handing the in-progress rebase to an agent to resolve...",
                "WARN:".yellow()
            );
            if let Err(e) = self.resolve_conflicts(&issue_dir, &repo_dir, repo, issue_number).await {
                abort_in_progress_operations(&repo_dir)?;
                self.record(&issue_dir, &main_sha, RebaseOutcome::Conflict)?;
                println!(
                    "  {} Agent could not resolve the conflicts ({}) - rebase aborted, PR #{} left untouched and needs a manual rebase.",
                    "FAILED:".red().bold(),
                    e,
                    pr.number
                );
                github
                    .post_comment(
                        issue_number,
                        &format!(
                            "🔀 `{}` moved to `{}`, but automatically rebasing `{}` hit merge conflicts \
                             and the agent sent to resolve them failed ({}), so the rebase was aborted \
                             and PR #{} was left untouched. It needs a manual rebase before the dev \
                             build can be refreshed.",
                            main_ref,
                            short_sha(&main_sha),
                            branch,
                            e,
                            pr.number
                        ),
                    )
                    .await?;
                return Ok(RebaseOutcome::Conflict);
            }
            result.head_after = rev_parse(&repo_dir, "HEAD")?;
            println!("  {} Agent resolved the conflicts and completed the rebase.", "OK:".green().bold());
        }

        if result.head_after == result.head_before && !result.conflicted {
            self.record(&issue_dir, &main_sha, RebaseOutcome::AlreadyCurrent)?;
            println!("  {} {} already contained {}; no rebase needed.", "OK:".green(), branch, main_ref);
            self.publish(github, issue_number, &issue_dir, &repo_dir, &branch).await;
            return Ok(RebaseOutcome::AlreadyCurrent);
        }

        force_push(&repo_dir, &branch)?;
        // Recorded before publishing so a build failure never re-triggers the rebase.
        self.record(&issue_dir, &main_sha, RebaseOutcome::Rebased)?;
        let ahead = commits_ahead(&repo_dir, &main_ref).unwrap_or_else(|| "?".to_string());
        println!(
            "  {} Rebased {} commit(s) onto {} and force-pushed - PR #{} is up to date ({} -> {})",
            "OK:".green().bold(),
            ahead,
            short_sha(&main_sha),
            pr.number,
            short_sha(&result.head_before),
            short_sha(&result.head_after)
        );

        self.publish(github, issue_number, &issue_dir, &repo_dir, &branch).await;

        Ok(RebaseOutcome::Rebased)
    }

    /// Asks an agent to resolve an in-progress, conflicted rebase for
    /// `issue_number`, reusing the same agent configured for `ai-pr-creation`
    /// (the state that already resolves conflicts hit while landing a PR).
    async fn resolve_conflicts(&self, issue_dir: &Path, repo_dir: &Path, repo: &str, issue_number: u64) -> Result<()> {
        let agent_config = self.workspace.read_config()?;
        let agent_command = agent_config.get_command_for_state("ai-pr-creation");

        let audit_dir = self.workspace.get_audit_dir(repo, issue_number)?;
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let audit_log_path = audit_dir.join(format!("auto_rebase_conflict_resolution_{}.log", timestamp));

        resolve_conflicts_with_agent(repo_dir, issue_dir, audit_log_path, &self.merge_conflict_keyword, agent_command)
            .await
    }

    /// A merged PR means this issue's work now lives in `main`, so the build a
    /// human should install is main's. The checkout is moved onto the latest
    /// `origin/<main>` and published once per merged PR, producing the
    /// `*-main.apk` that supersedes the branch's dev build.
    async fn publish_main_after_merge(
        &self,
        github: &GithubClient,
        issue_number: u64,
        issue_dir: &Path,
        repo_dir: &Path,
        branch: &str,
    ) -> Result<RebaseOutcome> {
        let Some(pr) = self.merged_pr(repo_dir, branch)? else {
            return Ok(RebaseOutcome::Skipped);
        };

        let state = read_rebase_state(issue_dir);
        if !is_new_merged_pr(state.as_ref(), pr.number) {
            return Ok(RebaseOutcome::Skipped);
        }

        let main = &self.config.main_branch;
        let main_sha = rev_parse(repo_dir, &format!("origin/{}", main))?;
        println!(
            "{} PR #{} ({}) was merged - moving the checkout to {} ({}) for a fresh build",
            tag(),
            pr.number,
            branch,
            main,
            short_sha(&main_sha).yellow()
        );
        if !pr.url.is_empty() {
            println!("  {} {}", "PR:".blue(), pr.url);
        }

        abort_in_progress_operations(repo_dir)?;
        if !working_tree_clean(repo_dir)? {
            println!(
                "  {} Uncommitted changes in {:?}; leaving the checkout on {}.",
                "WARN:".yellow(),
                repo_dir,
                branch
            );
            return Ok(RebaseOutcome::Skipped);
        }

        if let Err(e) = switch_to_main(repo_dir, main) {
            println!("  {} {}", "WARN:".yellow(), e);
            return Ok(RebaseOutcome::Skipped);
        }
        println!("  {} Checked out {} at {}", "OK:".green(), main, short_sha(&main_sha));

        // Left unrecorded on a failed build so the next pass retries it.
        if self.publish(github, issue_number, issue_dir, repo_dir, main).await == PublishOutcome::Failed {
            return Ok(RebaseOutcome::Skipped);
        }
        record_merged(issue_dir, &main_sha, pr.number)?;
        Ok(RebaseOutcome::Merged)
    }

    async fn publish(
        &self,
        github: &GithubClient,
        issue_number: u64,
        issue_dir: &Path,
        repo_dir: &Path,
        branch: &str,
    ) -> PublishOutcome {
        publish_and_report(
            github,
            issue_number,
            issue_dir,
            repo_dir,
            branch,
            &self.config.publish_script,
        )
        .await
    }

    /// Records the handled main sha, preserving whatever head was last published.
    fn record(&self, issue_dir: &Path, main_sha: &str, outcome: RebaseOutcome) -> Result<()> {
        let previous = read_rebase_state(issue_dir);
        write_rebase_state(
            issue_dir,
            &RebaseState {
                last_main_sha: main_sha.to_string(),
                outcome,
                last_published_sha: previous.as_ref().and_then(|s| s.last_published_sha.clone()),
                merged_pr: previous.and_then(|s| s.merged_pr),
            },
        )
    }

    fn open_pr(&self, repo_dir: &Path, branch: &str) -> Result<Option<OpenPr>> {
        pr_for_branch(repo_dir, branch, "open")
    }

    /// The most recent merged PR for `branch`; `gh` lists newest first.
    fn merged_pr(&self, repo_dir: &Path, branch: &str) -> Result<Option<OpenPr>> {
        pr_for_branch(repo_dir, branch, "merged")
    }
}

#[cfg(test)]
#[path = "rebase_tests.rs"]
mod rebase_tests;
