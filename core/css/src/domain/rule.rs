use crate::domain::declaration::DeclarationList;
use crate::domain::error::CssError;
use crate::domain::selector::Selector;

/// A CSS rule combining one or more selectors with a block of declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    selectors: Vec<Selector>,
    declarations: DeclarationList,
}

impl Rule {
    /// Creates a new CSS rule, enforcing that at least one selector is provided.
    ///
    /// # Errors
    /// Returns `CssError::InvalidSelector` if `selectors` is empty.
    pub fn new(selectors: Vec<Selector>, declarations: DeclarationList) -> Result<Self, CssError> {
        if selectors.is_empty() {
            return Err(CssError::InvalidSelector(
                "CSS rule must contain at least one selector".to_string(),
            ));
        }
        Ok(Self {
            selectors,
            declarations,
        })
    }

    /// Accesses the rule selectors.
    #[must_use]
    pub fn selectors(&self) -> &[Selector] {
        &self.selectors
    }

    /// Accesses the rule declaration list.
    #[must_use]
    pub const fn declarations(&self) -> &DeclarationList {
        &self.declarations
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
