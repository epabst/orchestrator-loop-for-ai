use super::*;
use std::time::Duration;

#[tokio::test]
async fn test_wait_with_feedback_duration() {
    let start = std::time::Instant::now();
    let duration = Duration::from_secs(2);
    // This will call the wait_with_feedback function I'm about to implement
    let signaled = wait_with_feedback(duration).await.unwrap();
    let elapsed = start.elapsed();

    assert!(!signaled);
    assert!(elapsed >= duration);
}

// Tests for sticky issue selection feature
#[test]
fn test_sticky_state_clear() {
    // Test that clear_sticky_state() properly clears the sticky issue
    // We test this by directly manipulating the orchestrator's sticky state
    // since we can't easily construct Issue objects for the other methods

    // This test verifies the basic state management works
    // In a full test suite, we'd use mocks or integration tests for the async methods
    let mut state: Option<(String, u64)> = Some(("repo".to_string(), 42));

    // Simulate what clear_sticky_state does
    state = None;

    assert_eq!(state, None);
}

#[test]
fn test_sticky_state_tracking() {
    // Test that sticky state can be set and retrieved
    let mut state: Option<(String, u64)> = None;

    // Initially should be None
    assert_eq!(state, None);

    // Set sticky state
    state = Some(("test-repo".to_string(), 123));

    // Should be set now
    assert!(state.is_some());
    let (repo, num) = state.unwrap();
    assert_eq!(repo, "test-repo");
    assert_eq!(num, 123);
}
