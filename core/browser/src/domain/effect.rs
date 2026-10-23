//! What a [`crate::BrowsingContext`] asks the world to do.

use network::Url;

use crate::domain::{ContextId, FetchRequest, NavigationId};

/// An instruction from a browsing context to `alloy`'s executor. Every variant
/// names its context so a context tree (#90) can be routed without changing
/// this type.
#[derive(Clone, Debug, PartialEq)]
pub enum BrowserEffect {
    /// A navigation to `Url` began: the omnibox should show it.
    NavigationStarted(ContextId, Url),
    /// Fetch `FetchRequest` and report the result stamped with the
    /// [`NavigationId`].
    Fetch(FetchRequest, NavigationId),
    /// The document at `Url` replaced the previous one.
    DocumentLoaded(ContextId, Url),
    /// What the content area shows changed: it needs a relayout.
    ContentDamaged(ContextId),
}

impl BrowserEffect {
    /// The context the effect is for.
    #[must_use]
    pub const fn context(&self) -> ContextId {
        match self {
            Self::NavigationStarted(context, _)
            | Self::DocumentLoaded(context, _)
            | Self::ContentDamaged(context) => *context,
            Self::Fetch(_, navigation) => navigation.context(),
        }
    }
}
