//! [`matches`] — does one parsed selector select one snapshot node?
//!
//! Right to left, never recursive. The rightmost [`SelectorStep`] is the
//! **subject**: the element a match is about. It is tested first, and only if it
//! holds does the walk look leftward — which is why a page-wide sheet costs one
//! compound test per node instead of one subtree walk per rule.
//!
//! The walk is an explicit work stack of `(step index, candidate)` frames, the
//! same discipline as `application/snapshot.rs:22-31`: a selector nested a
//! thousand combinators deep is a heap allocation, never a blown stack.
//! Termination is structural — every frame pushed carries a **strictly smaller**
//! step index, so the walk cannot cycle.
//!
//! `:hover` / `:active` / `:focus` parse, weigh towards specificity, and never
//! match: a [`DomSnapshot`] projects elements, attributes and tree shape
//! (`PRD-007:35-36`), not interaction state, which the engine does not have
//! until the window and event phases.

use crate::domain::dom_snapshot::{DomSnapshot, NodeRef, SnapshotId, SnapshotNodeKind};
use crate::domain::selector::compound::{AttributeSelectors, IdentifierList, PseudoClasses};
use crate::domain::selector::{
    AttributeMatch, AttributeSelector, Combinator, ComplexSelector, CompoundSelector, PseudoClass,
    SelectorList, SelectorStep, TypeSelector,
};
use crate::domain::specificity::Specificity;

/// The `class` attribute, whose value is a whitespace-separated set rather than
/// one string (HTML §3.2.6.7).
const CLASS_ATTRIBUTE: &str = "class";
/// The `id` attribute a `#name` component is compared against.
const ID_ATTRIBUTE: &str = "id";

/// Whether `selector` selects `node`.
///
/// `node` travels by value: [`NodeRef`] is `Copy` and two words wide, so a
/// reference would only add an indirection (`trivially_copy_pass_by_ref`).
#[must_use]
pub fn matches(selector: &ComplexSelector, node: NodeRef<'_>, snapshot: &DomSnapshot) -> bool {
    let Some(subject) = selector.subject_index() else {
        return false;
    };
    let mut work: Vec<(usize, SnapshotId)> = vec![(subject, node.id())];
    while let Some((index, candidate)) = work.pop() {
        if step_reached_the_leftmost(selector, index, candidate, snapshot, &mut work) {
            return true;
        }
    }
    false
}

/// The strongest specificity among the selectors of `list` that match `node`, or
/// `None` when none of them does — the cascade's per-rule weight
/// (CSS Selectors L4 §17).
#[must_use]
pub fn strongest_match(
    list: &SelectorList,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
) -> Option<Specificity> {
    list.iter()
        .filter(|selector| matches(selector, node, snapshot))
        .map(ComplexSelector::specificity)
        .max()
}

/// Tests one frame. Answers whether the whole selector is satisfied; otherwise
/// it pushes the frames the combinator opens and answers `false`.
///
/// Command and query in one call is the deliberate exception this file makes,
/// for the same reason `infrastructure/parser/selectors.rs:11-15` makes it: the
/// alternative is re-deriving the frontier the caller just computed.
fn step_reached_the_leftmost(
    selector: &ComplexSelector,
    index: usize,
    candidate: SnapshotId,
    snapshot: &DomSnapshot,
    work: &mut Vec<(usize, SnapshotId)>,
) -> bool {
    let Some((step, node)) = frame(selector, index, candidate, snapshot) else {
        return false;
    };
    if !compound_matches(step.compound(), node, snapshot) {
        return false;
    }
    let Some(previous) = index.checked_sub(1) else {
        return true;
    };
    push_candidates(work, previous, step.combinator(), node, snapshot);
    false
}

/// The step and node one frame names, or `None` when either id is foreign.
fn frame<'selector, 'snapshot>(
    selector: &'selector ComplexSelector,
    index: usize,
    candidate: SnapshotId,
    snapshot: &'snapshot DomSnapshot,
) -> Option<(&'selector SelectorStep, NodeRef<'snapshot>)> {
    let step = selector.step(index)?;
    let node = snapshot.node(candidate)?;
    Some((step, node))
}

/// Pushes every node that could satisfy step `previous`, given that the step to
/// its right matched `node`.
fn push_candidates(
    work: &mut Vec<(usize, SnapshotId)>,
    previous: usize,
    combinator: Combinator,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
) {
    for candidate in candidates_for(combinator, node, snapshot) {
        work.push((previous, candidate));
    }
}

/// What each combinator means, read right to left.
fn candidates_for(
    combinator: Combinator,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
) -> Vec<SnapshotId> {
    match combinator {
        Combinator::Descendant => ancestors(node, snapshot),
        Combinator::Child => node.parent().into_iter().collect(),
        Combinator::NextSibling => nearest_previous_sibling(node, snapshot),
        Combinator::SubsequentSibling => previous_element_siblings(node, snapshot),
    }
}

/// Every ancestor of `node`, nearest first.
fn ancestors(node: NodeRef<'_>, snapshot: &DomSnapshot) -> Vec<SnapshotId> {
    let mut chain: Vec<SnapshotId> = Vec::new();
    let mut cursor = node.parent();
    while let Some(id) = cursor {
        chain.push(id);
        cursor = snapshot.node(id).and_then(NodeRef::parent);
    }
    chain
}

/// The one element sibling immediately before `node`, for `A + B`.
fn nearest_previous_sibling(node: NodeRef<'_>, snapshot: &DomSnapshot) -> Vec<SnapshotId> {
    let mut earlier = previous_element_siblings(node, snapshot);
    earlier.pop().into_iter().collect()
}

/// Every element sibling before `node`, in document order, for `A ~ B`.
fn previous_element_siblings(node: NodeRef<'_>, snapshot: &DomSnapshot) -> Vec<SnapshotId> {
    let siblings = element_siblings(node, snapshot);
    let position = position_among(&siblings, node).unwrap_or(0);
    siblings.into_iter().take(position).collect()
}

// ---- compound matching --------------------------------------------------

/// Every condition of one compound, against one node. Text and comment nodes
/// never match: a selector selects elements.
fn compound_matches(
    compound: &CompoundSelector,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
) -> bool {
    if node.kind() != SnapshotNodeKind::Element {
        return false;
    }
    type_matches(compound.type_selector(), node)
        && classes_match(compound.classes(), node)
        && ids_match(compound.ids(), node)
        && attributes_match(compound.attributes(), node)
        && pseudo_classes_match(compound.pseudo_classes(), node, snapshot)
}

fn type_matches(selector: &TypeSelector, node: NodeRef<'_>) -> bool {
    match selector {
        TypeSelector::Universal => true,
        TypeSelector::Named(name) => node.tag() == Some(name.as_str()),
    }
}

fn classes_match(classes: &IdentifierList, node: NodeRef<'_>) -> bool {
    let list = node.attribute(CLASS_ATTRIBUTE).unwrap_or_default();
    classes
        .iter()
        .all(|name| class_list_contains(list, name.as_str()))
}

/// The `class` attribute is a set of whitespace-separated names, so `.wide`
/// matches `class="tall wide"` but not `class="widescreen"`.
fn class_list_contains(list: &str, name: &str) -> bool {
    list.split_whitespace().any(|entry| entry == name)
}

fn ids_match(ids: &IdentifierList, node: NodeRef<'_>) -> bool {
    let actual = node.attribute(ID_ATTRIBUTE);
    ids.iter().all(|name| actual == Some(name.as_str()))
}

fn attributes_match(selectors: &AttributeSelectors, node: NodeRef<'_>) -> bool {
    selectors
        .iter()
        .all(|selector| attribute_matches(selector, node))
}

fn attribute_matches(selector: &AttributeSelector, node: NodeRef<'_>) -> bool {
    let Some(value) = node.attribute(selector.name().as_str()) else {
        return false;
    };
    match selector.match_kind() {
        AttributeMatch::Exists => true,
        AttributeMatch::Exact(expected) => value == expected,
    }
}

fn pseudo_classes_match(
    classes: &PseudoClasses,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
) -> bool {
    classes
        .iter()
        .all(|pseudo_class| pseudo_class_matches(pseudo_class, node, snapshot))
}

fn pseudo_class_matches(
    pseudo_class: PseudoClass,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
) -> bool {
    match pseudo_class {
        PseudoClass::Hover | PseudoClass::Active | PseudoClass::Focus => false,
        PseudoClass::FirstChild => element_index_one_based(node, snapshot) == Some(1),
        PseudoClass::LastChild => is_last_element_child(node, snapshot),
        PseudoClass::NthChild(formula) => nth_child_matches(formula, node, snapshot),
    }
}

fn nth_child_matches(
    formula: crate::domain::selector::NthFormula,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
) -> bool {
    element_index_one_based(node, snapshot).is_some_and(|index| formula.matches(index))
}

fn is_last_element_child(node: NodeRef<'_>, snapshot: &DomSnapshot) -> bool {
    let siblings = element_siblings(node, snapshot);
    siblings.last() == Some(&node.id())
}

/// `node`'s 1-based position among its **element** siblings — what
/// `:nth-child()` counts (CSS Selectors L4 §6.6.3 counts element children, and
/// text and comment nodes are not children for its purpose).
fn element_index_one_based(node: NodeRef<'_>, snapshot: &DomSnapshot) -> Option<u32> {
    let siblings = element_siblings(node, snapshot);
    let position = position_among(&siblings, node)?;
    u32::try_from(position.saturating_add(1)).ok()
}

fn position_among(siblings: &[SnapshotId], node: NodeRef<'_>) -> Option<usize> {
    siblings.iter().position(|id| *id == node.id())
}

/// Every element child of `node`'s parent, in document order. Empty when `node`
/// is the projected root, which has no parent to be a child of.
fn element_siblings(node: NodeRef<'_>, snapshot: &DomSnapshot) -> Vec<SnapshotId> {
    let Some(parent) = node.parent().and_then(|id| snapshot.node(id)) else {
        return Vec::new();
    };
    parent
        .children()
        .filter(|id| is_element(*id, snapshot))
        .collect()
}

fn is_element(id: SnapshotId, snapshot: &DomSnapshot) -> bool {
    snapshot
        .node(id)
        .is_some_and(|node| node.kind() == SnapshotNodeKind::Element)
}
