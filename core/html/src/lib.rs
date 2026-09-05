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
pub const PORT_SCHEMA_VERSION: u32 = 1;

pub use application::conformance::run_html_conformance;
pub use application::ports::{RawKind, TokenSink, TokenSinkResult, TreeSink};
pub use domain::error::HtmlError;
pub use domain::tag::{
    closes_list_item, closes_paragraph, is_block_tag, is_heading_tag, is_rawtext_tag, is_void_tag,
};
pub use domain::token::{AttributeEntry, AttributeList, DoctypeToken, TagToken, Token};
pub use infrastructure::dom_sink::DomTreeSink;
pub use infrastructure::mock::{MockEvent, MockTreeSink};
pub use infrastructure::tokenizer::Tokenizer;
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

/// Parses an HTML input string into a complete [`dom::DomTree`].
pub fn parse(html: &str) -> Result<dom::DomTree, HtmlError> {
    let mut sink = DomTreeSink::new();
    parse_with_sink(html, &mut sink)?;
    Ok(sink.into_tree())
}

/// Parses an HTML input string pumping events into an arbitrary [`TreeSink`].
pub fn parse_with_sink(html: &str, sink: &mut dyn TreeSink) -> Result<(), HtmlError> {
    let mut builder = TreeBuilder::new(sink);
    let tokenizer = Tokenizer::new(html);
    tokenizer.run(&mut builder)
}
