//! The cascade adapters.
//!
//! B1 shipped one — [`UaCascade`], re-exported from
//! [`crate::infrastructure::ua_sheet`] — plus the two halves it delegates to:
//! [`author_rules`] (matching, ordering and application) and [`values`]
//! (`DeclarationValue` → computed value). B2 (`plano:435-443`) is the real
//! three-origin cascade: `!important` wins, `assets/ua.css` replaces the
//! hard-coded UA defaults, and `initial` / `inherit` resolve here.

pub mod author_rules;
pub mod flex_values;
pub mod font_values;
pub mod grid_values;
pub mod logical_values;
pub mod overflow_values;
pub mod position_values;
pub mod sizing_constraints_values;
pub mod text_values;
pub mod values;
pub mod variable_values;
pub mod visual_values;

pub use crate::infrastructure::ua_sheet::UaCascade;
