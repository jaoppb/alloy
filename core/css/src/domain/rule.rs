use crate::domain::declaration::DeclarationList;
use crate::domain::selector::Selector;

/// A CSS rule combining one or more selectors with a block of declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: DeclarationList,
}

impl Rule {
    /// Creates a new CSS rule.
    #[must_use]
    pub const fn new(selectors: Vec<Selector>, declarations: DeclarationList) -> Self {
        Self {
            selectors,
            declarations,
        }
    }
}

/// First-class collection wrapping CSS rules (ADR-0010).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuleSet {
    rules: Vec<Rule>,
}

impl RuleSet {
    /// Creates an empty rule set.
    #[must_use]
    pub const fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Adds a rule to this set.
    pub fn push(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Returns the number of rules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Checks if the rule set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Iterates over the rules.
    pub fn iter(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter()
    }
}
