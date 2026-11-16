//! Decision records produced by the ai-development agent.
//!
//! The agent answers with two markdown sections, `## Existing decisions`
//! (non-obsolete ones carried forward) and `## New decisions`. The orchestrator
//! posts only the new ones on the issue, keeps the full list in the issue
//! workspace and puts all of them (new ones marked) in the PR description.

pub const EXISTING_HEADING: &str = "Existing decisions";
pub const NEW_HEADING: &str = "New decisions";
/// File in the issue workspace holding the current full decision list.
pub const DECISIONS_FILE: &str = "decisions.md";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Decisions {
    pub existing: Vec<String>,
    pub new: Vec<String>,
}

fn heading_text(line: &str) -> Option<String> {
    let t = line.trim();
    if !t.starts_with('#') {
        return None;
    }
    Some(t.trim_start_matches('#').trim().trim_end_matches(':').trim().to_lowercase())
}

fn bullet_text(line: &str) -> Option<String> {
    let t = line.trim();
    for p in ["- ", "* ", "• "] {
        if let Some(rest) = t.strip_prefix(p) {
            let rest = rest.trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// Extracts the decision sections from an agent response. Returns `None` when
/// neither heading is present, so callers can fall back to the raw output.
pub fn parse_decisions(response: &str) -> Option<Decisions> {
    #[derive(PartialEq)]
    enum Section { None, Existing, New }
    let mut section = Section::None;
    let mut found = false;
    let mut d = Decisions::default();
    for line in response.lines() {
        if let Some(h) = heading_text(line) {
            if h == EXISTING_HEADING.to_lowercase() {
                section = Section::Existing;
                found = true;
            } else if h == NEW_HEADING.to_lowercase() {
                section = Section::New;
                found = true;
            } else {
                section = Section::None;
            }
            continue;
        }
        if let Some(b) = bullet_text(line) {
            match section {
                Section::Existing => d.existing.push(b),
                Section::New => d.new.push(b),
                Section::None => {}
            }
        }
    }
    if found { Some(d) } else { None }
}

impl Decisions {
    /// All decisions, existing first, as the list that gets stored.
    pub fn all(&self) -> Vec<String> {
        self.existing.iter().chain(self.new.iter()).cloned().collect()
    }

    /// Issue-comment text: only the new decisions.
    pub fn format_comment(&self) -> String {
        if self.new.is_empty() {
            return format!("**{}:** none", NEW_HEADING);
        }
        let bullets: Vec<String> = self.new.iter().map(|b| format!("- {}", b)).collect();
        format!("**{}:**\n{}", NEW_HEADING, bullets.join("\n"))
    }

    /// Stored form: the same two sections the agent emits, so it can be fed back.
    pub fn format_stored(&self) -> String {
        format_pr_section_from(&self.existing, &self.new)
    }
}

fn format_pr_section_from(existing: &[String], new: &[String]) -> String {
    let mut out = String::from("## Decisions\n");
    for e in existing {
        out.push_str(&format!("- {}\n", e));
    }
    for n in new {
        out.push_str(&format!("- **(new)** {}\n", n));
    }
    out
}

/// PR-description section from a stored list. Items tagged `**(new)**` stay
/// marked as new; everything else is shown as an existing decision.
pub fn format_pr_section(stored: &str) -> String {
    let items: Vec<String> = stored.lines().filter_map(bullet_text).collect();
    if items.is_empty() {
        return String::new();
    }
    let mut out = String::from("## Decisions\n");
    for i in items {
        out.push_str(&format!("- {}\n", i.replace("**(new)**", "**(new)**")));
    }
    out
}

/// The agent's text before its first decision heading (its summary).
pub fn summary_before_decisions(response: &str) -> String {
    let mut out = Vec::new();
    for line in response.lines() {
        if let Some(h) = heading_text(line) {
            if h == EXISTING_HEADING.to_lowercase() || h == NEW_HEADING.to_lowercase() {
                break;
            }
        }
        out.push(line);
    }
    out.join("\n").trim().to_string()
}

/// Reads decisions back from a stored file: new ones carry the `(new)` mark
/// and become existing for the next invocation.
pub fn carry_forward(stored: &str) -> Vec<String> {
    stored
        .lines()
        .filter_map(bullet_text)
        .map(|b| b.trim_start_matches("**(new)**").trim().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "intro\n## Existing decisions\n- Keep A\n- Keep B\n\n## New decisions\n- Add C\n* Add D\n\nCOMPLETED_DEVELOPMENT";

    #[test]
    fn parses_both_sections() {
        let d = parse_decisions(SAMPLE).unwrap();
        assert_eq!(d.existing, vec!["Keep A", "Keep B"]);
        assert_eq!(d.new, vec!["Add C", "Add D"]);
    }

    #[test]
    fn parse_fails_without_headings() {
        assert!(parse_decisions("just a summary\n- bullet").is_none());
    }

    #[test]
    fn comment_has_only_new() {
        let c = parse_decisions(SAMPLE).unwrap().format_comment();
        assert!(c.contains("Add C") && c.contains("Add D"));
        assert!(!c.contains("Keep A"));
    }

    #[test]
    fn stored_marks_new_and_pr_section_has_all() {
        let stored = parse_decisions(SAMPLE).unwrap().format_stored();
        let pr = format_pr_section(&stored);
        assert!(pr.contains("- Keep A"));
        assert!(pr.contains("- **(new)** Add C"));
    }

    #[test]
    fn carry_forward_strips_new_mark() {
        let stored = parse_decisions(SAMPLE).unwrap().format_stored();
        assert_eq!(carry_forward(&stored), vec!["Keep A", "Keep B", "Add C", "Add D"]);
    }

    #[test]
    fn summary_stops_at_first_decision_heading() {
        assert_eq!(summary_before_decisions(SAMPLE), "intro");
    }

    #[test]
    fn empty_new_comment() {
        let d = Decisions { existing: vec!["x".into()], new: vec![] };
        assert!(d.format_comment().contains("none"));
        assert_eq!(d.all(), vec!["x"]);
    }
}
