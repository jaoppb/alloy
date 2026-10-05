//! `html` — HTML5 tokenization and tree construction over [`dom::DomTree`].
//!
//! Provides the [`TokenSink`] and [`TreeSink`] replaceable ports (PRD-008, ADR-0011)
//! and a streaming tokenizer complying with WHATWG HTML5 §13.2.5.
//!
//! ## Contract record
//!
//! This crate is the `TokenSink` / `TreeSink` port under the `ADR-0011` Replaceable Port
//! Contract. `docs/architecture/html-tree-sink-port-contract.md` records the state of all
//! seven items. A change to [`Token`] or the `TreeSink`/`TokenSink` signatures bumps
//! [`PORT_SCHEMA_VERSION`] and adds a migration note to `PRD-008`.

#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod application;
pub mod domain;
pub mod infrastructure;

/// The observable version of this port's token and tree-sink vocabulary.
///
/// `ADR-0011` item 3. Introduced in v0.5 Phase P, recording the surface v0.5
/// Phase B5 shipped — see `docs/architecture/html-tree-sink-port-contract.md`.
pub const PORT_SCHEMA_VERSION: u32 = 2;

pub use application::conformance::run_html_conformance;
pub use application::ports::{RawKind, ScriptDescriptor, TokenSink, TokenSinkResult, TreeSink};
pub use domain::attribute::{
    AttributeEntry, AttributeList, AttributeName, AttributeValue, DuplicateAttribute,
};
pub use domain::diagnostic::{Diagnostics, ParseDiagnostic, ParseErrorCode};
pub use domain::entity::HtmlEntity;
pub use domain::error::{HtmlError, InvalidAttributeName, InvalidTagName};
pub use domain::handle::NodeHandle;
pub use domain::location::SourceLocation;
pub use domain::tag::TagName;
pub use domain::text::Text;
pub use domain::token::{DoctypeToken, TagToken, Token};
#[cfg(feature = "dom")]
pub use infrastructure::dom_sink::{DomTreeSink, ParseOutcome};
pub use infrastructure::mock::{MockEvent, MockTreeSink};
pub use infrastructure::tokenizer::{Tokenizer, TokenizerRunResult};
pub use infrastructure::tree_builder::TreeBuilder;

/// Declared HTML tags supported by this implementation in the v0.5 cut.
pub const SUPPORTED_TAGS: &[&str] = &[
    "a",
    "article",
    "blockquote",
    "body",
    "br",
    "code",
    "div",
    "em",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "img",
    "li",
    "link",
    "main",
    "meta",
    "nav",
    "noscript",
    "ol",
    "p",
    "pre",
    "script",
    "section",
    "span",
    "strong",
    "style",
    "title",
    "ul",
];

/// Declared HTML syntactic constructs verified by the conformance manifest.
pub const SUPPORTED_SYNTAX: &[&str] = &[
    "<!DOCTYPE html>",
    "<tag attr=\"val\">",
    "<tag attr='val'>",
    "<tag attr=val>",
    "<tag bool-attr>",
    "<tag />",
    "<!-- comment -->",
    "&entity; named entity",
    "&#decimal; numeric entity",
    "&#xhex; numeric entity",
    "<script> rawtext",
    "<style> rawtext",
    "p tag omission",
    "li tag omission",
    "void tags auto-close",
];

/// Declared parse-error codes the tokenizer and tree builder report, verified by the manifest.
pub const SUPPORTED_PARSE_ERRORS: &[&str] = ParseErrorCode::CODES;

/// Parses an HTML input string into a [`ParseOutcome`]: the tree plus every recoverable parse
/// error met on the way. Only a sink or constructor failure is an `Err` (ADR-0023).
#[cfg(feature = "dom")]
pub fn parse(html: &str) -> Result<ParseOutcome, HtmlError> {
    let mut sink = DomTreeSink::new();
    parse_with_sink(html, &mut sink)?;
    Ok(sink.into_outcome())
}

/// Parses an HTML input string pumping events into an arbitrary [`TreeSink`].
///
/// Recoverable errors are delivered to [`TreeSink::parse_error`]; the sink owns them.
pub fn parse_with_sink<S: TreeSink + ?Sized>(html: &str, sink: &mut S) -> Result<(), HtmlError> {
    let mut builder = TreeBuilder::new(sink);
    let tokenizer = Tokenizer::new(html);
    tokenizer.run(&mut builder)
}
