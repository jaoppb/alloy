//! Which browsing context a navigation, fetch result or region belongs to.

use std::fmt;

/// Names one browsing context.
///
/// Carried by every `NavigationId` (`core/browser`), [`crate::RegionId`],
/// `FetchResult` and `BrowserEffect` so a context tree (#90) can route them
/// without a signature change — the browser is *addressable, not nested* today
/// (#110 decision 14): only [`ContextId::ROOT`] exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContextId(u32);

impl ContextId {
    /// The one context a window shows today.
    pub const ROOT: Self = Self(0);
}

impl fmt::Display for ContextId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "context#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ContextId;

    #[test]
    fn the_root_context_names_itself() {
        assert_eq!(ContextId::ROOT.to_string(), "context#0");
    }
}
