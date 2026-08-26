use crate::domain::rule::RuleSet;

/// Represents a parsed CSS stylesheet document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleSheet {
    rules: RuleSet,
}

impl StyleSheet {
    /// Creates a new stylesheet from a rule set.
    #[must_use]
    pub const fn new(rules: RuleSet) -> Self {
        Self { rules }
    }

    /// Accesses the inner rule set.
    #[must_use]
    pub const fn rules(&self) -> &RuleSet {
        &self.rules
    }

    /// Accesses the inner rule set mutably.
    pub fn rules_mut(&mut self) -> &mut RuleSet {
        &mut self.rules
    }
}
