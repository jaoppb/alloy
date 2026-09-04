//! The cascade adapters.
//!
//! B1 ships one — [`UaCascade`], re-exported from
//! [`crate::infrastructure::ua_sheet`] — plus the two halves it now delegates
//! to: [`author_rules`] (matching, ordering and application) and [`values`]
//! (`DeclarationValue` → computed value). B2 grows the real three-origin
//! cascade with `!important` and full unit resolution here.

pub mod author_rules;
pub mod values;

pub use crate::infrastructure::ua_sheet::UaCascade;
