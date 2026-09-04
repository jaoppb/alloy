//! The hand-written CSS Syntax Level 3 parser (`relatório §2.8:336-338`).
//!
//! Parsing is **native Rust and not a port** (`PRD-007:11-13`): nothing in this
//! module appears in a [`crate::CascadeResolver`] signature. Its product is the
//! boundary aggregate [`StyleSheetSet`] that `application/ports.rs` already
//! receives.
//!
//! The pipeline is four stages, each in its own file:
//!
//! ```text
//! &str → scanner::Scanner → tokenizer::tokenize → token::TokenStream
//!      → rules::parse_rule_list (selectors.rs, media.rs, values.rs) → StyleSheetSet
//! ```
//!
//! [`tokenize`] is **total** — no input makes it fail. Recovery lives one level
//! up, in `rules.rs`, and every recovery leaves a
//! [`crate::ParseNote`]. The only failure that escapes as a `CssError` is a
//! source nested past `rules::MAX_NESTING_DEPTH`, which is hostile input rather
//! than a stylesheet.

pub mod media;
pub mod rules;
pub mod scanner;
pub mod selectors;
pub mod token;
pub mod tokenizer;
pub mod values;

pub use token::{SpannedToken, Token, TokenStream};
pub use tokenizer::tokenize;

use crate::domain::declaration::DeclarationBlock;
use crate::domain::error::CssError;
use crate::domain::stylesheet_set::{Origin, StyleSheetSet};

/// Parses a whole stylesheet — the text of a `<style>` element, a `.css`
/// subresource, or the embedded user-agent sheet — into rules tagged `origin`.
///
/// Everything the parser recovered from is in
/// [`StyleSheetSet::notes`](crate::StyleSheetSet::notes); the `Err` is reserved
/// for a source that cannot be read at all.
pub fn parse_stylesheet(source: &str, origin: Origin) -> Result<StyleSheetSet, CssError> {
    let mut tokens = tokenize(source);
    let mut sheets = StyleSheetSet::new();
    rules::parse_rule_list(&mut tokens, origin, &mut sheets)?;
    Ok(sheets)
}

/// Parses the body of a `style=` attribute: declarations with no selector and
/// no braces (CSS Style Attributes §3).
pub fn parse_inline_style(source: &str) -> Result<DeclarationBlock, CssError> {
    let mut notes = StyleSheetSet::new();
    parse_inline_style_recording(source, &mut notes)
}

/// The same parse, with the recovery notes appended to `sheets` — how
/// [`crate::collect_style_sheets`] keeps one document's notes together.
pub(crate) fn parse_inline_style_recording(
    source: &str,
    sheets: &mut StyleSheetSet,
) -> Result<DeclarationBlock, CssError> {
    let mut tokens = tokenize(source);
    rules::read_declaration_block(&mut tokens, sheets)
}
