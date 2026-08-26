use crate::domain::token::{HtmlError, HtmlToken};
use crate::domain::tokenizer::HtmlTokenizer;
use crate::domain::tree_builder::TreeBuilder;
use dom::DomTree;

/// Application service parsing raw HTML into a structured `DomTree`.
pub struct HtmlParser;

impl HtmlParser {
    /// Parses an HTML string slice into a complete `DomTree`.
    ///
    /// # Errors
    /// Returns `HtmlError` if a lexical or DOM tree building error occurs.
    pub fn parse(html: &str) -> Result<DomTree, HtmlError> {
        let mut tokenizer = HtmlTokenizer::new(html);
        let mut builder = TreeBuilder::new();

        loop {
            let token = tokenizer.next_token()?;
            let is_eof = matches!(token, HtmlToken::Eof);
            builder.process_token(token)?;

            if is_eof {
                break;
            }
        }

        Ok(builder.finish())
    }
}

/// Convenience function parsing an HTML string into a `DomTree`.
///
/// # Errors
/// Returns `HtmlError` if parsing fails.
pub fn parse_html(html: &str) -> Result<DomTree, HtmlError> {
    HtmlParser::parse(html)
}
