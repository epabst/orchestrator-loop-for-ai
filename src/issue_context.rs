use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct IssueContext {
    pub issue_url: String,
    #[allow(dead_code)]
    pub issue_id: Option<String>,
    pub is_processing: bool,
    #[allow(dead_code)]
    pub timestamp: u64,
}

pub struct IssueContextStore {
    current_issue: Mutex<Option<IssueContext>>,
}

impl IssueContextStore {
    pub fn new() -> Self {
        Self {
            current_issue: Mutex::new(None),
        }
    }

    pub fn set_current_issue(&self, issue_url: String, issue_id: Option<String>) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let context = IssueContext {
            issue_url,
            issue_id,
            is_processing: true,
            timestamp,
        };

        let mut current = self.current_issue.lock().unwrap();
        *current = Some(context);
    }

    pub fn get_current_issue(&self) -> Option<IssueContext> {
        let current = self.current_issue.lock().unwrap();
        current.clone()
    }

    pub fn clear_current_issue(&self) {
        let mut current = self.current_issue.lock().unwrap();
        *current = None;
    }

    #[allow(dead_code)]
    pub fn is_processing(&self) -> bool {
        let current = self.current_issue.lock().unwrap();
        current.is_some()
    }
}

impl Default for IssueContextStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_store_is_empty() {
        let store = IssueContextStore::new();
        assert!(!store.is_processing());
        assert!(store.get_current_issue().is_none());
    }

    #[test]
    fn test_set_and_get_issue() {
        let store = IssueContextStore::new();
        store.set_current_issue(
            "https://github.com/owner/repo/issues/123".to_string(),
            Some("123".to_string()),
        );

        assert!(store.is_processing());
        let issue = store.get_current_issue();
        assert!(issue.is_some());
        let issue = issue.unwrap();
        assert_eq!(issue.issue_url, "https://github.com/owner/repo/issues/123");
        assert_eq!(issue.issue_id, Some("123".to_string()));
        assert!(issue.is_processing);
    }

    #[test]
    fn test_clear_issue() {
        let store = IssueContextStore::new();
        store.set_current_issue(
            "https://github.com/owner/repo/issues/123".to_string(),
            Some("123".to_string()),
        );
        assert!(store.is_processing());

        store.clear_current_issue();
        assert!(!store.is_processing());
        assert!(store.get_current_issue().is_none());
    }

    #[test]
    fn test_overwrite_issue() {
        let store = IssueContextStore::new();
        store.set_current_issue(
            "https://github.com/owner/repo/issues/123".to_string(),
            Some("123".to_string()),
        );

        store.set_current_issue(
            "https://github.com/owner/repo/issues/456".to_string(),
            Some("456".to_string()),
        );

        let issue = store.get_current_issue();
        assert!(issue.is_some());
        let issue = issue.unwrap();
        assert_eq!(issue.issue_url, "https://github.com/owner/repo/issues/456");
        assert_eq!(issue.issue_id, Some("456".to_string()));
    }

    #[test]
    fn test_set_issue_without_id() {
        let store = IssueContextStore::new();
        store.set_current_issue(
            "https://github.com/owner/repo/issues/789".to_string(),
            None,
        );

        let issue = store.get_current_issue();
        assert!(issue.is_some());
        let issue = issue.unwrap();
        assert_eq!(issue.issue_url, "https://github.com/owner/repo/issues/789");
        assert_eq!(issue.issue_id, None);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let store = Arc::new(IssueContextStore::new());
        let mut handles = vec![];

        for i in 0..5 {
            let store_clone = Arc::clone(&store);
            let handle = thread::spawn(move || {
                let url = format!("https://github.com/owner/repo/issues/{}", i);
                store_clone.set_current_issue(url.clone(), Some(i.to_string()));
                let issue = store_clone.get_current_issue();
                assert!(issue.is_some());
                let issue = issue.unwrap();
                assert_eq!(issue.issue_url, url);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
