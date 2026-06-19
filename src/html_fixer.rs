//! HTML Structure Repair Module
//!
//! This module provides HTML structure repair functionality to fix malformed HTML documents.
//! It addresses the root cause of progressive font-size scaling issues caused by improperly
//! nested or unclosed HTML elements.
//!
//! # Problem Statement
//!
//! When HTML documents have unclosed block-level elements (particularly `<p>` tags), the browser's
//! HTML parser auto-corrects by inferring tag closure. This can result in unintended nested structures:
//!
//! ```html
//! <p>Text
//! <p>More text    <!-- Still inside first <p> due to auto-correction -->
//! </p>
//! ```
//!
//! This nesting causes CSS font-size inheritance to compound through nested elements, resulting in
//! progressively larger text from top to bottom of the page.
//!
//! # Solution
//!
//! The HtmlFixer module prevents this by:
//! 1. Tracking all opening and closing tags with a stack
//! 2. Automatically closing unclosed tags at appropriate boundaries
//! 3. Preventing paragraph nesting by closing previous `<p>` tags when new ones open
//! 4. Moving block elements outside of `<p>` containers
//! 5. Removing orphaned closing tags
//! 6. Preserving all content, attributes, and semantic structure
//!
//! # Usage Example
//!
//! ```rust
//! use orchestrator_loop_for_ai::html_fixer::HtmlFixer;
//!
//! let fixer = HtmlFixer::new();
//! let malformed_html = "<p>First paragraph\n<p>Second paragraph</p>";
//! let fixed_html = fixer.fix(malformed_html);
//! // Result: "<p>First paragraph</p>\n<p>Second paragraph</p>"
//! ```
//!
//! # Design Principles
//!
//! - **Minimal Intervention**: Only modifies HTML structure; CSS styling is unchanged
//! - **Content Preservation**: All original content, links, and attributes are retained
//! - **Semantic Correctness**: Maintains proper HTML5 semantic element nesting
//! - **Validation-Driven**: Output is designed to pass HTML5 validation

use regex::Regex;

/// HtmlFixer repairs malformed HTML documents by fixing structural issues.
///
/// This fixer handles the following types of repairs:
/// - Closing unclosed paragraph (`<p>`) and block elements
/// - Flattening nested paragraph tags to siblings
/// - Moving block elements out of paragraph containers
/// - Preserving all content, attributes, and hyperlinks
/// - Handling orphaned closing tags
/// - Respecting HTML5 self-closing elements
///
/// The fixer maintains a tag stack to track opening and closing tags,
/// ensuring proper nesting and preventing cascading font-size inheritance issues.
pub struct HtmlFixer {
    block_elements: Vec<&'static str>,
    self_closing_elements: Vec<&'static str>,
}

impl HtmlFixer {
    /// Creates a new HtmlFixer instance.
    pub fn new() -> Self {
        HtmlFixer {
            block_elements: vec![
                "div", "p", "section", "article", "nav", "aside", "header", "footer",
                "main", "blockquote", "pre", "ul", "ol", "li", "dl", "dt", "dd",
                "table", "thead", "tbody", "tfoot", "tr", "td", "th", "form",
                "fieldset", "legend", "figure", "figcaption", "h1", "h2", "h3",
                "h4", "h5", "h6",
            ],
            self_closing_elements: vec![
                "img", "br", "hr", "input", "meta", "link", "area", "base",
                "col", "embed", "source", "track", "wbr",
            ],
        }
    }

    /// Fixes HTML structure issues in the provided HTML string.
    ///
    /// This method performs the following repairs:
    /// 1. Closes all unclosed tags by maintaining a tag stack
    /// 2. Prevents nested `<p>` tags by closing the previous `<p>` when a new one opens
    /// 3. Moves block elements outside of `<p>` tags by closing the `<p>` first
    /// 4. Removes orphaned closing tags that don't have matching opening tags
    /// 5. Preserves all content, attributes, and inline elements
    ///
    /// # Arguments
    /// * `html` - A string slice containing the HTML to fix
    ///
    /// # Returns
    /// A String containing the fixed HTML with proper tag balance and nesting
    pub fn fix(&self, html: &str) -> String {
        if html.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        let mut tag_stack: Vec<String> = Vec::new();
        let tag_regex = Regex::new(r#"(<[^>]+>)"#).unwrap();
        let last_pos = std::cell::RefCell::new(0);

        for tag_match in tag_regex.find_iter(html) {
            let tag_str = tag_match.as_str();
            let start = tag_match.start();

            // Add content before tag
            let content_before = &html[*last_pos.borrow()..start];
            result.push_str(content_before);

            if self.is_closing_tag(tag_str) {
                let tag_name = self.extract_tag_name(tag_str);

                // Handle orphaned closing tags
                if let Some(pos) = tag_stack.iter().rposition(|t| t == &tag_name) {
                    // Close all tags from top of stack until we find the matching opening tag
                    while tag_stack.len() > pos {
                        if let Some(unclosed) = tag_stack.pop() {
                            result.push_str(&format!("</{}>", unclosed));
                        }
                    }
                    result.push_str(tag_str);
                } else {
                    // Orphaned closing tag - skip it
                    // (we don't add it because there's no matching opening tag)
                }
            } else if self.is_opening_tag(tag_str) {
                let tag_name = self.extract_tag_name(tag_str).to_lowercase();

                // Handle nested <p> tags - close previous <p> if we're opening a new one
                if tag_name == "p" {
                    if let Some(pos) = tag_stack.iter().rposition(|t| t == "p") {
                        // Close the unclosed <p> tag
                        while tag_stack.len() > pos {
                            if let Some(unclosed) = tag_stack.pop() {
                                result.push_str(&format!("</{}>", unclosed));
                            }
                        }
                    }
                }

                // Handle block elements inside <p> tags - close <p> first
                if self.is_block_element(&tag_name) && tag_name != "p" {
                    while let Some(open_tag) = tag_stack.last() {
                        if open_tag == "p" {
                            tag_stack.pop();
                            result.push_str("</p>");
                        } else {
                            break;
                        }
                    }
                }

                if !self.is_self_closing(&tag_name) {
                    tag_stack.push(tag_name);
                }

                result.push_str(tag_str);
            } else {
                // Comment or other tag-like content
                result.push_str(tag_str);
            }

            *last_pos.borrow_mut() = tag_match.end();
        }

        // Add remaining content
        if *last_pos.borrow() < html.len() {
            result.push_str(&html[*last_pos.borrow()..]);
        }

        // Close any remaining unclosed tags in reverse order
        while let Some(tag) = tag_stack.pop() {
            result.push_str(&format!("</{}>", tag));
        }

        result
    }

    fn is_opening_tag(&self, tag: &str) -> bool {
        tag.starts_with('<') && !tag.starts_with("</") && !tag.starts_with("<!") && !tag.ends_with("/>")
    }

    fn is_closing_tag(&self, tag: &str) -> bool {
        tag.starts_with("</") && tag.ends_with('>')
    }

    fn extract_tag_name(&self, tag: &str) -> String {
        let tag = tag.trim_start_matches('<').trim_end_matches('>').trim_end_matches('/');

        // Extract tag name (first word before space or >)
        let tag_name = tag
            .split_whitespace()
            .next()
            .unwrap_or(tag)
            .to_lowercase();

        tag_name
    }

    fn is_block_element(&self, tag: &str) -> bool {
        self.block_elements.contains(&tag.to_lowercase().as_str())
    }

    fn is_self_closing(&self, tag: &str) -> bool {
        self.self_closing_elements.contains(&tag.to_lowercase().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
