use super::*;
use chrono::{Datelike, Timelike};

#[test]
fn detects_session_limit_with_named_timezone_reset() {
    let err = detect("You've hit your session limit · resets 1:20pm (America/Denver)")
        .expect("should detect a rate limit");
    let reset = err.reset_at.expect("should parse a reset time");
    assert_eq!(reset.hour(), 13);
    assert_eq!(reset.minute(), 20);
}

#[test]
fn detects_weekly_limit() {
    assert!(detect("You've hit your weekly limit").is_some());
}

#[test]
fn detects_bare_429_with_default_cooldown() {
    let err = detect("API Error: Request rejected (429)").expect("should detect a rate limit");
    assert!(err.reset_at.is_none());
    assert_eq!(err.wait_duration(), DEFAULT_COOLDOWN);
}

#[test]
fn parses_pipe_delimited_epoch_form() {
    let err = detect("Claude AI usage limit reached|1735500000").expect("should detect a rate limit");
    let reset = err.reset_at.expect("should parse an epoch reset time");
    assert_eq!(reset.year(), 2024, "2024-12-29 19:20:00 UTC, per the fixed epoch above");
}

#[test]
fn ignores_ordinary_failures() {
    assert!(detect("Agent failed with status exit code: 1").is_none());
    assert!(detect("error: could not resolve merge conflict").is_none());
}

#[test]
fn wait_duration_adds_safety_margin_and_never_goes_negative_for_a_past_reset() {
    let err = RateLimitError {
        message: "You've hit your session limit".to_string(),
        reset_at: Some(chrono::Local::now() - chrono::Duration::minutes(5)),
    };
    // A reset time already in the past still yields a short, positive wait
    // rather than an error or a zero-length busy loop.
    assert!(err.wait_duration() > std::time::Duration::from_secs(0));
    assert!(err.wait_duration() <= SAFETY_MARGIN);
}
