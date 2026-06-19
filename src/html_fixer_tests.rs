#[cfg(test)]
mod tests {
    use crate::html_fixer::HtmlFixer;

    #[test]
    fn test_close_unclosed_paragraph_tags() {
        let html = "<p>Some text\n<p>More text</p>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        // Should have balanced tags
        assert_eq!(result.matches("<p>").count(), result.matches("</p>").count());
        assert!(result.contains("Some text"));
        assert!(result.contains("More text"));
    }

    #[test]
    fn test_close_unclosed_tags_at_end() {
        let html = "<p>Paragraph text without closing tag";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        assert!(result.contains("</p>"));
        assert_eq!(result.matches("<p>").count(), result.matches("</p>").count());
    }

    #[test]
    fn test_flatten_nested_paragraphs() {
        let html = "<p>Outer\n<p>Inner</p>\n</p>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        // Nested <p> should be converted to siblings
        assert!(!result.contains("<p>") || result.matches("<p>").count() == result.matches("</p>").count());
        assert!(result.contains("Outer"));
        assert!(result.contains("Inner"));
    }

    #[test]
    fn test_move_block_elements_out_of_paragraph() {
        let html = "<p>Text <div>Block content</div> More text</p>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        // Should properly close <p> before <div>
        assert!(result.contains("</p>"));
        assert!(result.contains("<div>"));
        assert!(result.contains("</div>"));
        assert!(result.contains("Text"));
        assert!(result.contains("Block content"));
        assert!(result.contains("More text"));
    }

    #[test]
    fn test_preserve_content_and_links() {
        let html = "<p>Check out <a href=\"http://example.com\">this link</a>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        assert!(result.contains("Check out"));
        assert!(result.contains("this link"));
        assert!(result.contains("http://example.com"));
        assert!(result.contains("</a>"));
        assert!(result.contains("</p>"));
    }

    #[test]
    fn test_balanced_tags_simple() {
        let html = "<div><p>Test</p></div>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        assert_eq!(result.matches("<div>").count(), result.matches("</div>").count());
        assert_eq!(result.matches("<p>").count(), result.matches("</p>").count());
    }

    #[test]
    fn test_empty_document() {
        let html = "";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        assert_eq!(result, "");
    }

    #[test]
    fn test_preserve_attributes() {
        let html = "<div class=\"container\" id=\"main\"><p>Content</p>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        assert!(result.contains("class=\"container\""));
        assert!(result.contains("id=\"main\""));
        assert!(result.contains("</div>"));
    }

    #[test]
    fn test_multiple_unclosed_paragraphs() {
        let html = "<p>First\n<p>Second\n<p>Third</p>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        assert_eq!(result.matches("<p>").count(), result.matches("</p>").count());
        assert!(result.contains("First"));
        assert!(result.contains("Second"));
        assert!(result.contains("Third"));
    }

    #[test]
    fn test_orphaned_closing_tags() {
        let html = "<p>Text</p></div></div>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        // Opening and closing tags should be balanced
        assert_eq!(result.matches("<div>").count(), result.matches("</div>").count());
    }

    #[test]
    fn test_preserve_code_blocks() {
        let html = "<p>Code: <code>let x = 5;</code>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        assert!(result.contains("<code>"));
        assert!(result.contains("</code>"));
        assert!(result.contains("let x = 5;"));
        assert!(result.contains("</p>"));
    }

    #[test]
    fn test_semantic_elements_preserved() {
        let html = "<section><article><p>Content";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        assert!(result.contains("<section>"));
        assert!(result.contains("</section>"));
        assert!(result.contains("<article>"));
        assert!(result.contains("</article>"));
        assert!(result.contains("<p>"));
        assert!(result.contains("</p>"));
    }

    #[test]
    fn test_self_closing_tags_preserved() {
        let html = "<p>Image: <img src=\"test.jpg\" alt=\"test\"> End</p>";
        let fixer = HtmlFixer::new();
        let result = fixer.fix(html);

        assert!(result.contains("src=\"test.jpg\""));
        assert!(result.contains("alt=\"test\""));
        assert!(result.contains("</p>"));
    }
}
