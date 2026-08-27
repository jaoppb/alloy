#![forbid(unsafe_code)]

//! # Core CSS (`core/css`)
//!
//! CSS parser, selector matcher, specificity calculator, and style cascade engine.
//! Part of the aggregate rendering pipeline for Alloy (PRD-001, ADR-0010).

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::cascade::{DEFAULT_CASCADE_SCRIPT, StyleCascade};
pub use domain::computed::ComputedStyle;
pub use domain::declaration::{Declaration, DeclarationList};
pub use domain::error::CssError;
pub use domain::parser::{CssParser, parse_css};
pub use domain::ports::ColorResolver;
pub use domain::property::{Color, DisplayType, PropertyName, PropertyValue, Px};
pub use domain::rule::{Rule, RuleSet};
pub use domain::selector::{AttributeMatcher, PseudoClass, Selector};
pub use domain::specificity::Specificity;
pub use domain::styled_node::{StyledNode, StyledTree};
pub use domain::stylesheet::StyleSheet;
pub use infrastructure::color_resolver::CssColorResolver;
