use super::*;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git_ok(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run git {:?}: {}", args, e));
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Creates a bare origin plus a working clone with a single commit on `main`.
fn setup_repo(root: &Path) -> std::path::PathBuf {
    let origin = root.join("origin.git");
    git_ok(root, &["init", "--bare", "-b", "main", origin.to_str().unwrap()]);

    let repo = root.join("repo");
    git_ok(root, &["clone", origin.to_str().unwrap(), repo.to_str().unwrap()]);
    git_ok(&repo, &["config", "user.email", "test@example.com"]);
    git_ok(&repo, &["config", "user.name", "Test User"]);

    fs::write(repo.join("shared.txt"), "line one\n").unwrap();
    git_ok(&repo, &["add", "."]);
    git_ok(&repo, &["commit", "-m", "initial"]);
    git_ok(&repo, &["push", "-u", "origin", "main"]);
    repo
}

/// Adds a commit to `main` on the origin so `origin/main` moves ahead.
fn advance_main(repo: &Path, file: &str, contents: &str) {
    git_ok(repo, &["checkout", "main"]);
    fs::write(repo.join(file), contents).unwrap();
    git_ok(repo, &["add", "."]);
    git_ok(repo, &["commit", "-m", "advance main"]);
    git_ok(repo, &["push", "origin", "main"]);
}

fn commit_on_branch(repo: &Path, branch: &str, file: &str, contents: &str) {
    git_ok(repo, &["checkout", "-B", branch, "main"]);
    fs::write(repo.join(file), contents).unwrap();
    git_ok(repo, &["add", "."]);
    git_ok(repo, &["commit", "-m", "branch work"]);
}

#[test]
fn test_is_new_main_sha_without_prior_state() {
    assert!(is_new_main_sha(None, "abc123"));
}

#[test]
fn test_is_new_main_sha_matches_recorded_sha() {
    let state = RebaseState { last_main_sha: "abc123".to_string(), outcome: RebaseOutcome::Rebased };
    assert!(!is_new_main_sha(Some(&state), "abc123"));
    assert!(is_new_main_sha(Some(&state), "def456"));
}

#[test]
fn test_is_new_main_sha_retries_after_conflict_only_when_main_moves() {
    let state = RebaseState { last_main_sha: "abc123".to_string(), outcome: RebaseOutcome::Conflict };
    assert!(!is_new_main_sha(Some(&state), "abc123"), "a conflict must not be retried for the same main sha");
    assert!(is_new_main_sha(Some(&state), "def456"));
}

#[test]
fn test_rebase_state_roundtrip() {
    let dir = TempDir::new().unwrap();
    assert_eq!(read_rebase_state(dir.path()), None);

    let state = RebaseState { last_main_sha: "deadbeef".to_string(), outcome: RebaseOutcome::Conflict };
    write_rebase_state(dir.path(), &state).unwrap();

    assert_eq!(read_rebase_state(dir.path()), Some(state));
}

#[test]
fn test_read_rebase_state_ignores_corrupt_file() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(REBASE_STATE_FILE), "not json").unwrap();
    assert_eq!(read_rebase_state(dir.path()), None);
}

#[test]
fn test_parse_pr_number_from_gh_output() {
    assert_eq!(parse_pr_number("[{\"number\":42}]").unwrap(), Some(42));
    assert_eq!(parse_pr_number("[]").unwrap(), None);
    assert_eq!(parse_pr_number("").unwrap(), None);
    assert_eq!(parse_pr_number("   \n").unwrap(), None);
}

#[test]
fn test_rebase_branch_onto_moves_branch_onto_new_main() {
    let tmp = TempDir::new().unwrap();
    let repo = setup_repo(tmp.path());
    commit_on_branch(&repo, "issue-1", "feature.txt", "feature\n");
    advance_main(&repo, "other.txt", "other\n");

    fetch_origin(&repo).unwrap();
    let result = rebase_branch_onto(&repo, "issue-1", "origin/main").unwrap();

    assert!(!result.conflicted);
    assert_ne!(result.head_before, result.head_after, "the branch should have been replayed onto the new main");
    // Both the branch work and the new main commit are present.
    assert!(repo.join("feature.txt").exists());
    assert!(repo.join("other.txt").exists());
    assert_eq!(rev_parse(&repo, "HEAD").unwrap(), result.head_after);
}

#[test]
fn test_rebase_branch_onto_is_noop_when_already_current() {
    let tmp = TempDir::new().unwrap();
    let repo = setup_repo(tmp.path());
    commit_on_branch(&repo, "issue-1", "feature.txt", "feature\n");

    fetch_origin(&repo).unwrap();
    let result = rebase_branch_onto(&repo, "issue-1", "origin/main").unwrap();

    assert!(!result.conflicted);
    assert_eq!(result.head_before, result.head_after, "an up-to-date branch must not be rewritten");
}

#[test]
fn test_rebase_branch_onto_aborts_on_conflict() {
    let tmp = TempDir::new().unwrap();
    let repo = setup_repo(tmp.path());
    commit_on_branch(&repo, "issue-1", "shared.txt", "branch version\n");
    advance_main(&repo, "shared.txt", "main version\n");

    fetch_origin(&repo).unwrap();
    let result = rebase_branch_onto(&repo, "issue-1", "origin/main").unwrap();

    assert!(result.conflicted);
    assert_eq!(result.head_before, result.head_after, "a conflicted rebase must leave the branch untouched");
    assert!(!rebase_in_progress(&repo), "the conflicted rebase must be aborted");
    assert!(working_tree_clean(&repo).unwrap());
}

#[test]
fn test_working_tree_clean_detects_uncommitted_changes() {
    let tmp = TempDir::new().unwrap();
    let repo = setup_repo(tmp.path());
    assert!(working_tree_clean(&repo).unwrap());

    fs::write(repo.join("shared.txt"), "dirty\n").unwrap();
    assert!(!working_tree_clean(&repo).unwrap());
}

#[test]
fn test_branch_sync_status_transitions() {
    let tmp = TempDir::new().unwrap();
    let repo = setup_repo(tmp.path());
    commit_on_branch(&repo, "issue-1", "feature.txt", "feature\n");

    assert_eq!(branch_sync_status(&repo, "issue-1").unwrap(), BranchSync::NoRemote);

    git_ok(&repo, &["push", "-u", "origin", "issue-1"]);
    fetch_origin(&repo).unwrap();
    assert_eq!(branch_sync_status(&repo, "issue-1").unwrap(), BranchSync::InSync);

    fs::write(repo.join("feature.txt"), "more\n").unwrap();
    git_ok(&repo, &["add", "."]);
    git_ok(&repo, &["commit", "-m", "local only"]);
    assert_eq!(branch_sync_status(&repo, "issue-1").unwrap(), BranchSync::LocalAhead);

    git_ok(&repo, &["reset", "--hard", "HEAD~2"]);
    assert_eq!(branch_sync_status(&repo, "issue-1").unwrap(), BranchSync::LocalBehind);

    fs::write(repo.join("divergent.txt"), "divergent\n").unwrap();
    git_ok(&repo, &["add", "."]);
    git_ok(&repo, &["commit", "-m", "divergent"]);
    assert_eq!(branch_sync_status(&repo, "issue-1").unwrap(), BranchSync::Diverged);
}

#[test]
fn test_branch_exists_locally() {
    let tmp = TempDir::new().unwrap();
    let repo = setup_repo(tmp.path());
    assert!(!branch_exists_locally(&repo, "issue-1").unwrap());

    commit_on_branch(&repo, "issue-1", "feature.txt", "feature\n");
    assert!(branch_exists_locally(&repo, "issue-1").unwrap());
}

#[test]
fn test_run_publish_script_executes_repo_script() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    let script = repo.join("publish-android-dev.sh");
    fs::write(&script, "#!/bin/bash\nset -e\necho published > published.txt\n").unwrap();
    make_executable(&script);

    assert!(run_publish_script(repo, "publish-android-dev.sh").unwrap());
    assert_eq!(fs::read_to_string(repo.join("published.txt")).unwrap().trim(), "published");
}

#[test]
fn test_run_publish_script_skips_when_missing() {
    let tmp = TempDir::new().unwrap();
    assert!(!run_publish_script(tmp.path(), "publish-android-dev.sh").unwrap());
}

#[test]
fn test_run_publish_script_reports_failure() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    let script = repo.join("publish-android-dev.sh");
    fs::write(&script, "#!/bin/bash\nexit 3\n").unwrap();
    make_executable(&script);

    let err = run_publish_script(repo, "publish-android-dev.sh").unwrap_err();
    assert!(err.to_string().contains("publish-android-dev.sh"), "unexpected error: {}", err);
}


/// End-to-end happy path: origin/main moves, the PR branch is rebased, force-pushed,
/// and the publish script runs. Ignored by default because it puts a `gh` stub on
/// PATH, which is process-global; run it with:
///   cargo test -- --ignored --test-threads=1
#[tokio::test]
#[ignore]
async fn test_process_issue_rebases_pushes_and_publishes() {
    let tmp = TempDir::new().unwrap();
    let workspace_root = tmp.path().join("ai-workspaces");
    let issue_dir = workspace_root.join("demo-7");
    fs::create_dir_all(&issue_dir).unwrap();

    // ~/ai-workspaces/demo-7/demo is the checkout the rebase must happen in.
    let origin = tmp.path().join("origin.git");
    git_ok(tmp.path(), &["init", "--bare", "-b", "main", origin.to_str().unwrap()]);
    let repo = issue_dir.join("demo");
    git_ok(tmp.path(), &["clone", origin.to_str().unwrap(), repo.to_str().unwrap()]);
    git_ok(&repo, &["config", "user.email", "test@example.com"]);
    git_ok(&repo, &["config", "user.name", "Test User"]);
    fs::write(repo.join("shared.txt"), "line one\n").unwrap();
    fs::write(
        repo.join("publish-android-dev.sh"),
        "#!/bin/bash\nset -e\necho \"$(git rev-parse --abbrev-ref HEAD)\" > \"$PUBLISH_MARKER\"\n",
    )
    .unwrap();
    make_executable(&repo.join("publish-android-dev.sh"));
    git_ok(&repo, &["add", "."]);
    git_ok(&repo, &["commit", "-m", "initial"]);
    git_ok(&repo, &["push", "-u", "origin", "main"]);

    commit_on_branch(&repo, "issue-7", "feature.txt", "feature\n");
    git_ok(&repo, &["push", "-u", "origin", "issue-7"]);
    advance_main(&repo, "other.txt", "other\n");
    git_ok(&repo, &["checkout", "issue-7"]);

    // Stub `gh` so the PR-existence check reports an open PR.
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let gh = bin_dir.join("gh");
    fs::write(&gh, "#!/bin/bash\necho '[{\"number\":99}]'\n").unwrap();
    make_executable(&gh);
    let marker = tmp.path().join("published.txt");
    std::env::set_var("PATH", format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()));
    std::env::set_var("PUBLISH_MARKER", &marker);

    let config = crate::config::AutoRebaseConfig::default();
    let workspace = Workspace { base_dir: workspace_root };
    let github = GithubClient::new("fake-token".to_string(), "owner".to_string(), "demo".to_string()).unwrap();
    let rebaser = AutoRebaser::new(&config, &workspace);

    let outcome = rebaser.process_issue(&github, "demo", 7).await.unwrap();
    assert_eq!(outcome, RebaseOutcome::Rebased);

    // The branch was replayed onto the new main and pushed there.
    assert!(repo.join("other.txt").exists());
    assert!(repo.join("feature.txt").exists());
    fetch_origin(&repo).unwrap();
    assert_eq!(branch_sync_status(&repo, "issue-7").unwrap(), BranchSync::InSync);

    // The publish script ran from the repo root on the issue branch.
    assert_eq!(fs::read_to_string(&marker).unwrap().trim(), "issue-7");

    // The handled sha is recorded, so a second pass is a no-op.
    let state = read_rebase_state(&issue_dir).unwrap();
    assert_eq!(state.outcome, RebaseOutcome::Rebased);
    assert_eq!(state.last_main_sha, rev_parse(&repo, "origin/main").unwrap());
    fs::remove_file(&marker).unwrap();
    assert_eq!(rebaser.process_issue(&github, "demo", 7).await.unwrap(), RebaseOutcome::Skipped);
    assert!(!marker.exists(), "an unchanged origin/main must not trigger another publish");
}

fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}
