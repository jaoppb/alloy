//! The cascade adapters.
//!
//! B1 shipped one — [`UaCascade`], re-exported from
//! [`crate::infrastructure::ua_sheet`] — plus the two halves it delegates to:
//! [`author_rules`] (matching, ordering and application) and [`values`]
//! (`DeclarationValue` → computed value). B2 (`plano:435-443`) is the real
//! three-origin cascade: `!important` wins, `assets/ua.css` replaces the
//! hard-coded UA defaults, and `initial` / `inherit` resolve here.

pub mod author_rules;
pub mod values;

pub use crate::infrastructure::ua_sheet::UaCascade;
