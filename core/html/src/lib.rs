#![forbid(unsafe_code)]

//! # Core HTML (`core/html`)
//!
//! HTML5 tokenizer and tree builder constructing `DomTree` aggregates from text streams.
//! Part of the aggregate rendering pipeline for Alloy (PRD-001, ADR-0010).

pub mod application;
pub mod domain;

pub use application::parser::{HtmlParser, parse_html};
pub use domain::entities::decode_html_entities;
pub use domain::token::{HtmlError, HtmlToken};
pub use domain::tokenizer::HtmlTokenizer;
pub use domain::tree_builder::{TreeBuilder, is_void_element};
