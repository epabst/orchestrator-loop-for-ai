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
