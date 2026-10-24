//! What a [`crate::BrowsingContext`] asks the world to do.

use network::Url;
use region::ContextId;

use crate::domain::{FetchRequest, NavigationId};

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

/// The effects one command produced, in the order to execute them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Effects(Vec<BrowserEffect>);

impl Effects {
    /// No effects.
    #[must_use]
    pub const fn none() -> Self {
        Self(Vec::new())
    }

    /// Appends `effect` after the ones already queued.
    pub fn push(&mut self, effect: BrowserEffect) {
        self.0.push(effect);
    }

    /// The effects in execution order.
    #[must_use]
    pub fn as_slice(&self) -> &[BrowserEffect] {
        &self.0
    }

    /// The effects in execution order, by reference.
    pub fn iter(&self) -> std::slice::Iter<'_, BrowserEffect> {
        self.0.iter()
    }

    /// How many effects there are.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the command asked for nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<BrowserEffect> for Effects {
    fn from_iter<T: IntoIterator<Item = BrowserEffect>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for Effects {
    type Item = BrowserEffect;
    type IntoIter = std::vec::IntoIter<BrowserEffect>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Effects {
    type Item = &'a BrowserEffect;
    type IntoIter = std::slice::Iter<'a, BrowserEffect>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
