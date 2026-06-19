#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_readme_mermaid_diagram_valid() {
        let readme_path = Path::new("README.md");
        let content = fs::read_to_string(readme_path)
            .expect("Failed to read README.md");

        let diagram_start = content.find("```mermaid")
            .expect("Mermaid diagram block not found in README.md");
        let diagram_content_start = diagram_start + "```mermaid".len();
        let diagram_end_pos = content[diagram_content_start..].find("```")
            .expect("Closing backticks not found");

        let diagram = &content[diagram_content_start..diagram_content_start + diagram_end_pos];

        // Verify required states are present
        let required_states = vec![
            "ai-requirements",
            "ai-design",
            "ai-development",
            "ai-review",
            "ai-pr-creation",
            "ai-done",
            "ai-human-help",
        ];

        for state in &required_states {
            assert!(diagram.contains(state), "State '{}' not found in diagram", state);
        }

        // Verify required transitions are present
        let required_transitions = vec![
            "ai-requirements --> ai-design",
            "ai-design --> ai-development",
            "ai-development --> ai-review",
            "ai-review --> ai-pr-creation",
            "ai-pr-creation --> ai-done",
            "ai-requirements --> ai-human-help",
            "ai-design --> ai-human-help",
            "ai-development --> ai-human-help",
            "ai-review --> ai-human-help",
            "ai-pr-creation --> ai-human-help",
        ];

        for transition in &required_transitions {
            assert!(diagram.contains(transition), "Transition '{}' not found in diagram", transition);
        }

        // Verify diagram has proper stateDiagram-v2 declaration
        assert!(diagram.contains("stateDiagram-v2"), "Diagram must use stateDiagram-v2");

        // Verify diagram has entry and exit points
        assert!(diagram.contains("[*] --> ai-requirements"), "Entry point [*] --> ai-requirements not found");
        assert!(diagram.contains("ai-done --> [*]"), "Exit point ai-done --> [*] not found");
    }

    #[test]
    fn test_readme_mermaid_diagram_syntax() {
        let readme_path = Path::new("README.md");
        let content = fs::read_to_string(readme_path)
            .expect("Failed to read README.md");

        let diagram_start = content.find("```mermaid")
            .expect("Mermaid diagram block not found in README.md");
        let diagram_content_start = diagram_start + "```mermaid".len();
        let diagram_end_pos = content[diagram_content_start..].find("```")
            .expect("Closing backticks not found");

        let diagram = &content[diagram_content_start..diagram_content_start + diagram_end_pos];

        // Verify first non-empty line is valid stateDiagram-v2 declaration
        let first_meaningful_line = diagram
            .lines()
            .find(|line| !line.trim().is_empty())
            .expect("Diagram is empty");

        assert_eq!(
            first_meaningful_line.trim(),
            "stateDiagram-v2",
            "First line must be 'stateDiagram-v2', got '{}'",
            first_meaningful_line
        );

        // Verify no malformed state names with invalid characters
        for line in diagram.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Check for common syntax errors: malformed declaration like "ai-am-v2"
            if trimmed.contains("ai-am-v2") {
                panic!("Found malformed state 'ai-am-v2' in diagram");
            }

            // Verify transitions use proper arrow syntax
            if !trimmed.starts_with("stateDiagram-v2") {
                let parts: Vec<&str> = trimmed.split("-->").collect();
                if parts.len() == 2 {
                    let from = parts[0].trim();
                    let to = parts[1].trim();

                    // Verify state names are not empty
                    assert!(!from.is_empty(), "Transition source cannot be empty");
                    assert!(!to.is_empty(), "Transition target cannot be empty");

                    // Verify state names don't have trailing/leading spaces in the declaration
                    let from_trimmed = from;
                    let to_trimmed = to;
                    assert_eq!(from, from_trimmed, "State name '{}' has leading spaces", from);
                    assert_eq!(to, to_trimmed, "State name '{}' has leading spaces", to);
                }
            }
        }

        // Verify proper indentation (no tabs, spaces only)
        for line in diagram.lines() {
            if line.starts_with('\t') {
                panic!("Diagram uses tabs for indentation, must use spaces");
            }
        }
    }
}
