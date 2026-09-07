use chrono::{DateTime, Local, TimeZone, Utc};
use regex::Regex;
use std::time::Duration;

/// A margin added on top of the parsed reset time, so a slightly-off clock or
/// a limit that resets a few seconds later than advertised doesn't send the
/// very next request straight back into another rate limit.
const SAFETY_MARGIN: Duration = Duration::from_secs(30);

/// Used when a limit message is recognized but carries no parseable reset
/// time (a bare "Request rejected (429)", for example). Long enough to stop
/// hammering the API, short enough that a real reset isn't missed for long.
const DEFAULT_COOLDOWN: Duration = Duration::from_secs(5 * 60);

/// Claude hit a usage/rate limit and printed so - as opposed to any other
/// agent failure, which should be surfaced and retried the normal way.
#[derive(Debug, Clone)]
pub struct RateLimitError {
    /// The agent's own message, kept for logging.
    pub message: String,
    /// When the limit resets, if the message named a time. `None` means the
    /// message was recognized as a limit but no time could be parsed from it.
    pub reset_at: Option<DateTime<Local>>,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message.trim())
    }
}

impl std::error::Error for RateLimitError {}

impl RateLimitError {
    /// How long to sleep before the orchestrator tries again.
    pub fn wait_duration(&self) -> Duration {
        match self.reset_at {
            Some(reset_at) => {
                let remaining = reset_at.signed_duration_since(Local::now());
                remaining.to_std().map(|d| d + SAFETY_MARGIN).unwrap_or(SAFETY_MARGIN)
            }
            None => DEFAULT_COOLDOWN,
        }
    }
}

/// Phrases Claude Code's `--print` mode is documented to emit (in the
/// `result` field, or on stderr for agents that don't speak stream-json) when
/// it can't make a request right now. Matched case-insensitively.
const LIMIT_PHRASES: [&str; 7] = [
    "hit your session limit",
    "hit your weekly limit",
    "hit your opus limit",
    "hit your sonnet limit",
    "usage limit reached",
    "spend limit reached",
    "request rejected (429)",
];

/// Scans agent output for a known rate/usage-limit message. Returns `None`
/// for ordinary failures, which should be retried the normal way rather than
/// paused on.
pub fn detect(text: &str) -> Option<RateLimitError> {
    let lower = text.to_lowercase();
    if !LIMIT_PHRASES.iter().any(|phrase| lower.contains(phrase)) {
        return None;
    }

    let reset_at = parse_named_reset(text).or_else(|| parse_epoch_reset(text));
    Some(RateLimitError { message: text.to_string(), reset_at })
}

/// Parses "resets 1:20pm (America/Denver)" style text into an absolute time,
/// using the named IANA timezone so a Denver reset is computed correctly
/// regardless of what timezone this machine is in.
fn parse_named_reset(text: &str) -> Option<DateTime<Local>> {
    let re = Regex::new(r"(?i)resets\s+(?:at\s+)?(\d{1,2})(?::(\d{2}))?\s*([ap]m)\s*\(([^)]+)\)").ok()?;
    let caps = re.captures(text)?;

    let hour12: u32 = caps.get(1)?.as_str().parse().ok()?;
    let minute: u32 = caps.get(2).map_or(Ok(0), |m| m.as_str().parse()).ok()?;
    let is_pm = caps.get(3)?.as_str().eq_ignore_ascii_case("pm");
    let tz_name = caps.get(4)?.as_str().trim();

    let hour24 = match (hour12 % 12, is_pm) {
        (h, true) => h + 12,
        (h, false) => h,
    };

    let tz: chrono_tz::Tz = tz_name.parse().ok()?;
    let now_in_tz = Utc::now().with_timezone(&tz);
    let candidate_naive = now_in_tz.date_naive().and_hms_opt(hour24, minute, 0)?;
    let mut candidate = match tz.from_local_datetime(&candidate_naive) {
        chrono::LocalResult::Single(dt) => dt,
        chrono::LocalResult::Ambiguous(dt, _) => dt,
        chrono::LocalResult::None => return None,
    };
    if candidate <= now_in_tz {
        candidate += chrono::Duration::days(1);
    }

    Some(candidate.with_timezone(&Local))
}

/// Parses the older "Claude AI usage limit reached|<epoch>" form some
/// non-interactive tooling emits, where `<epoch>` is Unix seconds or
/// milliseconds.
fn parse_epoch_reset(text: &str) -> Option<DateTime<Local>> {
    let re = Regex::new(r"(?i)usage limit reached\|(\d{10,13})").ok()?;
    let raw: i64 = re.captures(text)?.get(1)?.as_str().parse().ok()?;
    let secs = if raw > 10_000_000_000 { raw / 1000 } else { raw };
    let dt = DateTime::from_timestamp(secs, 0)?;
    Some(dt.with_timezone(&Local))
}

#[cfg(test)]
#[path = "rate_limit_tests.rs"]
mod rate_limit_tests;
