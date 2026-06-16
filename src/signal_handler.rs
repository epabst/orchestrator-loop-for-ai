use crate::issue_context::IssueContext;

pub struct OutputGenerator;

impl OutputGenerator {
    const ACTIVE_ISSUE_MESSAGE: &'static str = "Processing interrupted.\nIssue: {issue_url}\nPlease remove the 'ai-processing' label from this issue.";
    const IDLE_MESSAGE: &'static str = "";

    pub fn generate_output(context: Option<&IssueContext>) -> String {
        match context {
            Some(ctx) if ctx.is_processing => {
                Self::ACTIVE_ISSUE_MESSAGE.replace("{issue_url}", &ctx.issue_url)
            }
            _ => Self::IDLE_MESSAGE.to_string(),
        }
    }

    pub fn display_output(message: &str) {
        if !message.is_empty() {
            println!("{}", message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue_context::IssueContextStore;

    #[test]
    fn test_generate_output_with_active_issue() {
        let context = IssueContext {
            issue_url: "https://github.com/owner/repo/issues/123".to_string(),
            issue_id: Some("123".to_string()),
            is_processing: true,
            timestamp: 0,
        };

        let message = OutputGenerator::generate_output(Some(&context));
        assert!(message.contains("Processing interrupted"));
        assert!(message.contains("https://github.com/owner/repo/issues/123"));
        assert!(message.contains("'ai-processing'"));
    }

    #[test]
    fn test_generate_output_with_idle_state() {
        let message = OutputGenerator::generate_output(None);
        assert_eq!(message, "");
    }

    #[test]
    fn test_generate_output_message_format() {
        let context = IssueContext {
            issue_url: "https://github.com/test/repo/issues/42".to_string(),
            issue_id: Some("42".to_string()),
            is_processing: true,
            timestamp: 0,
        };

        let message = OutputGenerator::generate_output(Some(&context));
        let lines: Vec<&str> = message.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "Processing interrupted.");
        assert!(lines[1].starts_with("Issue:"));
        assert!(lines[2].contains("ai-processing"));
    }

    #[test]
    fn test_generate_output_with_non_processing_context() {
        let context = IssueContext {
            issue_url: "https://github.com/owner/repo/issues/123".to_string(),
            issue_id: Some("123".to_string()),
            is_processing: false,
            timestamp: 0,
        };

        let message = OutputGenerator::generate_output(Some(&context));
        assert_eq!(message, "");
    }

    #[test]
    fn test_display_output_with_message() {
        // This test verifies that display_output doesn't panic
        let message = "Test message";
        OutputGenerator::display_output(message);
    }

    #[test]
    fn test_display_output_with_empty_message() {
        // This test verifies that display_output handles empty messages
        let message = "";
        OutputGenerator::display_output(message);
    }

    #[test]
    fn test_integration_context_store_with_output_generator() {
        let store = IssueContextStore::new();
        store.set_current_issue(
            "https://github.com/owner/repo/issues/999".to_string(),
            Some("999".to_string()),
        );

        let context = store.get_current_issue();
        let message = OutputGenerator::generate_output(context.as_ref());

        assert!(message.contains("https://github.com/owner/repo/issues/999"));
        assert!(message.contains("ai-processing"));
    }

    #[test]
    fn test_integration_context_store_idle_with_output_generator() {
        let store = IssueContextStore::new();

        let context = store.get_current_issue();
        let message = OutputGenerator::generate_output(context.as_ref());

        assert_eq!(message, "");
    }
}
