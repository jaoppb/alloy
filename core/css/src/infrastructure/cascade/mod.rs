//! The cascade adapters. B0 ships one — [`UaCascade`], re-exported from
//! [`crate::infrastructure::ua_sheet`]. B2 grows a real three-origin cascade
//! with `!important` and unit resolution here.

pub use crate::infrastructure::ua_sheet::UaCascade;
