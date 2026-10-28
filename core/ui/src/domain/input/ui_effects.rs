//! [`UiEffects`] — the effects one input asks for, in order.

use super::UiEffect;

/// Zero or more [`UiEffect`]s, in the order the executor carries them out. One
/// click can both end address editing (damaging the chrome) and follow a link;
/// a single effect could not say both.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiEffects(Vec<UiEffect>);

impl UiEffects {
    /// No effect at all.
    #[must_use]
    pub const fn none() -> Self {
        Self(Vec::new())
    }

    /// `self` followed by `effect`. [`UiEffect::None`] is not kept.
    #[must_use]
    pub fn then(mut self, effect: UiEffect) -> Self {
        if effect != UiEffect::None {
            self.0.push(effect);
        }
        self
    }

    pub fn iter(&self) -> core::slice::Iter<'_, UiEffect> {
        self.0.iter()
    }
}

impl From<UiEffect> for UiEffects {
    fn from(effect: UiEffect) -> Self {
        Self::none().then(effect)
    }
}

impl IntoIterator for UiEffects {
    type Item = UiEffect;
    type IntoIter = std::vec::IntoIter<UiEffect>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'effects> IntoIterator for &'effects UiEffects {
    type Item = &'effects UiEffect;
    type IntoIter = core::slice::Iter<'effects, UiEffect>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
