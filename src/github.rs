use octocrab::Octocrab;
use octocrab::models::issues::Issue;
use octocrab::models::timelines::TimelineEvent;
use anyhow::Result;
use std::sync::Arc;

pub struct GithubClient {
    pub client: Arc<Octocrab>,
    pub owner: String,
    pub repo: String,
}

/// Splits `https://api.github.com/repos/<owner>/<repo>` into its owner and repo.
pub fn repo_slug(issue: &Issue) -> Result<(String, String)> {
    let path = issue.repository_url.path();
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 4 {
        return Err(anyhow::anyhow!("Unexpected repository URL: {}", issue.repository_url));
    }
    Ok((parts[2].to_string(), parts[3].to_string()))
}

/// Percent-encodes a label for use in a query string, so a label with spaces or
/// punctuation still round-trips. Keeps the crate free of an encoding dependency.
pub fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

impl GithubClient {
    pub fn new(token: String, owner: String, repo: String) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(token)
            .build()?;
        Ok(Self {
            client: Arc::new(client),
            owner,
            repo,
        })
    }

    pub fn for_repo(&self, owner: String, repo: String) -> Self {
        Self {
            client: self.client.clone(),
            owner,
            repo,
        }
    }

    pub async fn get_current_user(&self) -> Result<String> {
        let user = self.client.current().user().await?;
        Ok(user.login)
    }

    pub async fn list_assigned_issues(&self) -> Result<Vec<Issue>> {
        let user = self.get_current_user().await?;
        let issues = self.client.issues(&self.owner, &self.repo)
            .list()
            .assignee(user.as_str())
            .state(octocrab::params::State::Open)
            .send()
            .await?;
        Ok(issues.items)
    }

    pub async fn list_assigned_issues_in_all_repos(&self) -> Result<Vec<Issue>> {
        // GET /issues lists issues assigned to the authenticated user
        let issues: octocrab::Page<Issue> = self.client
            .get("/issues", None::<&()>)
            .await?;
        Ok(issues.items)
    }

    /// Assigned issues carrying `label`, open *or* closed. Closing an issue is
    /// how a human signals the work is over, but an issue waiting on its PR to
    /// merge still has orchestrator work left, so that state has to keep being
    /// polled after the close. The label keeps the query small.
    pub async fn list_assigned_issues_with_label(&self, label: &str) -> Result<Vec<Issue>> {
        let user = self.get_current_user().await?;
        let issues = self.client.issues(&self.owner, &self.repo)
            .list()
            .assignee(user.as_str())
            .labels(&[label.to_string()])
            .state(octocrab::params::State::All)
            .send()
            .await?;
        Ok(issues.items)
    }

    /// The all-repos twin of `list_assigned_issues_with_label`.
    pub async fn list_assigned_issues_with_label_in_all_repos(&self, label: &str) -> Result<Vec<Issue>> {
        let route = format!("/issues?filter=assigned&state=all&labels={}", percent_encode(label));
        let issues: octocrab::Page<Issue> = self.client.get(&route, None::<&()>).await?;
        Ok(issues.items)
    }

    pub async fn get_issue_comments(&self, issue_number: u64) -> Result<Vec<octocrab::models::issues::Comment>> {
        let comments = self.client.issues(&self.owner, &self.repo)
            .list_comments(issue_number)
            .per_page(10)
            .page(1u32)
            .send()
            .await?;
        Ok(comments.items)
    }

    pub async fn add_label(&self, issue_number: u64, label: &str) -> Result<()> {
        self.client.issues(&self.owner, &self.repo)
            .add_labels(issue_number, &[label.to_string()])
            .await?;
        Ok(())
    }

    pub async fn get_issue(&self, issue_number: u64) -> Result<Issue> {
        let issue = self.client.issues(&self.owner, &self.repo)
            .get(issue_number)
            .await?;
        Ok(issue)
    }

    pub async fn has_label(&self, issue_number: u64, label: &str) -> Result<bool> {
        let issue = self.client.issues(&self.owner, &self.repo)
            .get(issue_number)
            .await?;
        Ok(issue.labels.iter().any(|l| l.name == label))
    }

    pub async fn remove_label(&self, issue_number: u64, label: &str) -> Result<()> {
        self.client.issues(&self.owner, &self.repo)
            .remove_label(issue_number, label)
            .await?;
        Ok(())
    }

    pub async fn get_issue_events(&self, issue_number: u64) -> Result<Vec<TimelineEvent>> {
        let events = self.client.issues(&self.owner, &self.repo)
            .list_timeline_events(issue_number)
            .send()
            .await?;
        Ok(events.items)
    }

    pub async fn post_comment(&self, issue_number: u64, body: &str) -> Result<()> {
        self.client.issues(&self.owner, &self.repo)
            .create_comment(issue_number, body)
            .await?;
        Ok(())
    }

    pub async fn create_or_update_file(&self, path: &str, content: &str, message: &str, branch: &str) -> Result<()> {
        let repo_handler = self.client.repos(&self.owner, &self.repo);
        
        // Check if file exists to get sha
        let res = repo_handler
            .get_content()
            .path(path)
            .r#ref(branch)
            .send()
            .await;

        let sha = match res {
            Ok(content_items) => {
                if let Some(item) = content_items.items.first() {
                    Some(item.sha.clone())
                } else {
                    None
                }
            }
            Err(_) => None,
        };

        let builder = match sha {
            Some(s) => repo_handler.update_file(path, message, content, s),
            None => repo_handler.create_file(path, message, content),
        };

        builder.branch(branch).send().await?;
        Ok(())
    }
}
