//! The three [`crate::application::FontProvider`] adapters (v0.5 B3).

pub mod catalog;
pub mod synthetic;
pub mod system_provider;
pub mod ttf_provider;

pub use catalog::{FontCatalog, GenericFamily};
pub use synthetic::SyntheticFontProvider;
pub use system_provider::SystemFontProvider;
pub use ttf_provider::TtfParserProvider;
