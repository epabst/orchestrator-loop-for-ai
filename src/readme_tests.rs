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
}
