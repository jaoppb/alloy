pub mod entities;
pub mod token;
pub mod tokenizer;
pub mod tree_builder;

pub use entities::{HtmlEntity, decode_html_entities};
pub use token::{HtmlError, HtmlToken};
pub use tokenizer::HtmlTokenizer;
pub use tree_builder::{TreeBuilder, is_void_element};
