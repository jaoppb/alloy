//! Infrastructure adapters and implementations for HTML tokenization and tree construction.

pub mod dom_sink;
pub mod mock;
pub mod tokenizer;
pub mod tree_builder;

pub use dom_sink::DomTreeSink;
pub use mock::{MockEvent, MockTreeSink};
pub use tokenizer::Tokenizer;
pub use tree_builder::TreeBuilder;
