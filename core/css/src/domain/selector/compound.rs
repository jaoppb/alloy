//! [`CompoundSelector`] — everything that must hold of **one** element, and the
//! three first-class collections it is built from (`ADR-0010:129` — no public
//! `Vec`).
//!
//! `div#main.wide[data-role="nav"]:first-child` is one compound: a type
//! selector plus a set of classes, ids, attribute selectors and pseudo-classes,
//! in no particular order. A combinator separates two compounds; it never
//! appears inside one.

use core::fmt::{self, Display};

use crate::domain::identifier::Identifier;
use crate::domain::selector::component::{AttributeSelector, PseudoClass, TypeSelector};
use crate::domain::specificity::Specificity;

/// A run of identifiers in source order — the class names of a compound, or its
/// element ids. One collection serves both because they differ only in what
/// they are matched against, not in shape.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct IdentifierList {
    names: Vec<Identifier>,
}

impl IdentifierList {
    #[must_use]
    pub const fn new() -> Self {
        Self { names: Vec::new() }
    }

    pub fn push(&mut self, name: Identifier) {
        self.names.push(name);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Identifier> + '_ {
        self.names.iter()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.names.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// The `[…]` components of one compound, in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AttributeSelectors {
    selectors: Vec<AttributeSelector>,
}

impl AttributeSelectors {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selectors: Vec::new(),
        }
    }

    pub fn push(&mut self, selector: AttributeSelector) {
        self.selectors.push(selector);
    }

    pub fn iter(&self) -> impl Iterator<Item = &AttributeSelector> + '_ {
        self.selectors.iter()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.selectors.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.selectors.is_empty()
    }
}

/// The `:…` components of one compound, in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PseudoClasses {
    classes: Vec<PseudoClass>,
}

impl PseudoClasses {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            classes: Vec::new(),
        }
    }

    pub fn push(&mut self, pseudo_class: PseudoClass) {
        self.classes.push(pseudo_class);
    }

    pub fn iter(&self) -> impl Iterator<Item = PseudoClass> + '_ {
        self.classes.iter().copied()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.classes.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }
}

/// Every condition that must hold of one element at once.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct CompoundSelector {
    type_selector: TypeSelector,
    classes: IdentifierList,
    ids: IdentifierList,
    attributes: AttributeSelectors,
    pseudo_classes: PseudoClasses,
}

impl CompoundSelector {
    /// The universal compound `*` — no class, id, attribute or pseudo-class.
    #[must_use]
    pub const fn universal() -> Self {
        Self {
            type_selector: TypeSelector::Universal,
            classes: IdentifierList::new(),
            ids: IdentifierList::new(),
            attributes: AttributeSelectors::new(),
            pseudo_classes: PseudoClasses::new(),
        }
    }

    /// Replaces the type half. The parser calls this once, before any other
    /// component, because a type selector may only lead a compound.
    pub fn set_type_selector(&mut self, type_selector: TypeSelector) {
        self.type_selector = type_selector;
    }

    pub fn push_class(&mut self, name: Identifier) {
        self.classes.push(name);
    }

    pub fn push_id(&mut self, name: Identifier) {
        self.ids.push(name);
    }

    pub fn push_attribute(&mut self, selector: AttributeSelector) {
        self.attributes.push(selector);
    }

    pub fn push_pseudo_class(&mut self, pseudo_class: PseudoClass) {
        self.pseudo_classes.push(pseudo_class);
    }

    #[must_use]
    pub const fn type_selector(&self) -> &TypeSelector {
        &self.type_selector
    }

    #[must_use]
    pub const fn classes(&self) -> &IdentifierList {
        &self.classes
    }

    #[must_use]
    pub const fn ids(&self) -> &IdentifierList {
        &self.ids
    }

    #[must_use]
    pub const fn attributes(&self) -> &AttributeSelectors {
        &self.attributes
    }

    #[must_use]
    pub const fn pseudo_classes(&self) -> &PseudoClasses {
        &self.pseudo_classes
    }

    /// Whether the compound is exactly `*` — nothing beyond the universal type
    /// selector was written.
    #[must_use]
    pub const fn is_universal_only(&self) -> bool {
        matches!(self.type_selector, TypeSelector::Universal)
            && self.classes.is_empty()
            && self.ids.is_empty()
            && self.attributes.is_empty()
            && self.pseudo_classes.is_empty()
    }

    /// `(ids, classes + attributes + pseudo-classes, types)` — CSS Selectors
    /// L4 §17, the three-component specificity of `relatório §2.8:334`.
    #[must_use]
    pub fn specificity(&self) -> Specificity {
        let class_like = self
            .classes
            .len()
            .saturating_add(self.attributes.len())
            .saturating_add(self.pseudo_classes.len());
        Specificity::from_counts(self.ids.len(), class_like, 0)
            .plus(self.type_selector.specificity())
    }
}

impl fmt::Display for CompoundSelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_type_selector(self, formatter)?;
        for id in self.ids.iter() {
            write!(formatter, "#{id}")?;
        }
        for class in self.classes.iter() {
            write!(formatter, ".{class}")?;
        }
        for attribute in self.attributes.iter() {
            attribute.fmt(formatter)?;
        }
        self.pseudo_classes
            .iter()
            .try_for_each(|pseudo_class| pseudo_class.fmt(formatter))
    }
}

/// `*` is written only when it is the whole compound — `*.wide` and `.wide`
/// select the same elements, and the shorter form is the one an author wrote.
fn write_type_selector(
    compound: &CompoundSelector,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    let universal = matches!(compound.type_selector, TypeSelector::Universal);
    if universal && !compound.is_universal_only() {
        return Ok(());
    }
    compound.type_selector.fmt(formatter)
}
