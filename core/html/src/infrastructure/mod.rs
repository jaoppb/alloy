//! Infrastructure adapters and implementations for HTML tokenization and tree construction.

pub mod mock;
pub mod tokenizer;
pub mod tree_builder;

pub use mock::{MockEvent, MockTreeSink};
pub use tokenizer::{Tokenizer, TokenizerRunResult};
pub use tree_builder::TreeBuilder;
