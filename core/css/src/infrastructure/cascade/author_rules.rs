//! [`apply_author_rules`] — the matched declarations of a [`StyleSheetSet`],
//! applied to one node in cascade order (`plano:430-431`, `:435-443`).
//!
//! The unit that gets sorted is the **declaration**, not the rule: a single
//! rule can mix an `!important` declaration with a normal one
//! (`p { color: red !important; margin: 4px }`), and CSS Cascade L4 §4.2
//! ranks those two independently. Flattening to `(precedence, specificity,
//! source order, position in block)` before sorting is what keeps that case
//! correct without a second pass.
//!
//! The sort key is **total**: two distinct declarations can never tie,
//! because no two occupy the same `(order, position)` pair. That is what
//! keeps the 100-run determinism check of `PRD-007:100` green — there is no
//! residual ordering for a hash map or an unstable sort to decide.
//!
//! `precedence` is [`Origin::cascade_precedence`] — origin order for a normal
//! declaration, reversed for `!important` — so a `StyleSheetSet` carrying both
//! `Origin::UserAgent` and `Origin::Author` rules (`infrastructure/ua_sheet.rs`)
//! cascades them in one pass rather than two. The node's `style=` block is
//! still applied after every rule (CSS Cascade L4 §6.4.3): B2 leaves that
//! architectural choice from B1 as is, because no test in this cut exercises
//! an `!important` inline declaration against an `!important` rule.

use crate::application::matching::strongest_match;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::declaration::{Declaration, DeclarationBlock};
use crate::domain::dom_snapshot::{DomSnapshot, NodeRef};
use crate::domain::specificity::Specificity;
use crate::domain::stylesheet_set::{Origin, StyleRule, StyleSheetSet};
use crate::infrastructure::cascade::values::apply_declaration;

/// One declaration that selected the node, with the key it is ordered by.
struct MatchedDeclaration<'sheets> {
    precedence: u8,
    specificity: Specificity,
    order: usize,
    position: usize,
    declaration: &'sheets Declaration,
}

impl MatchedDeclaration<'_> {
    /// `(precedence, specificity, source order, position in block)` — CSS
    /// Cascade L4 §6.4, steps 3 through 6, with `!important` folded into
    /// `precedence` (§4.2) and the fourth field breaking a tie between two
    /// declarations of the very same rule.
    const fn sort_key(&self) -> (u8, Specificity, usize, usize) {
        (self.precedence, self.specificity, self.order, self.position)
    }
}

/// `base` with every matching declaration applied weakest-first, then the
/// node's `style=` block.
#[must_use]
pub(crate) fn apply_author_rules(
    base: ComputedStyle,
    parent: Option<&ComputedStyle>,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
    sheets: &StyleSheetSet,
) -> ComputedStyle {
    let matched = matched_declarations(node, snapshot, sheets);
    let cascaded = matched.iter().fold(base, |style, matched| {
        apply_one(style, parent, matched.declaration)
    });
    apply_inline_block(cascaded, parent, node, sheets)
}

/// Every declaration of `sheets` that selects `node`, in cascade order.
fn matched_declarations<'sheets>(
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
    sheets: &'sheets StyleSheetSet,
) -> Vec<MatchedDeclaration<'sheets>> {
    let mut matched: Vec<MatchedDeclaration<'sheets>> = sheets
        .rules()
        .enumerate()
        .flat_map(|(order, (origin, rule))| rule_declarations(order, origin, rule, node, snapshot))
        .collect();
    matched.sort_by_key(MatchedDeclaration::sort_key);
    matched
}

/// Every declaration of `rule`, each with its cascade key, or an empty list
/// when the rule does not select `node` — or still carries a `@media`
/// condition nobody evaluated.
///
/// A resolver receives no viewport (`PRD-007:56-60`, frozen at I3), so
/// skipping is the only safe reading of an unevaluated condition. The
/// producer discharges them first with [`StyleSheetSet::matching_viewport`].
fn rule_declarations<'sheets>(
    order: usize,
    origin: Origin,
    rule: &'sheets StyleRule,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
) -> Vec<MatchedDeclaration<'sheets>> {
    if !rule.media().is_always() {
        return Vec::new();
    }
    let Some(specificity) = strongest_match(rule.selectors(), node, snapshot) else {
        return Vec::new();
    };
    rule.declarations()
        .iter()
        .enumerate()
        .map(|(position, declaration)| MatchedDeclaration {
            precedence: origin.cascade_precedence(declaration.importance()),
            specificity,
            order,
            position,
            declaration,
        })
        .collect()
}

fn apply_block(
    style: ComputedStyle,
    parent: Option<&ComputedStyle>,
    block: &DeclarationBlock,
) -> ComputedStyle {
    block.iter().fold(style, |style, declaration| {
        apply_one(style, parent, declaration)
    })
}

/// A declaration whose value is outside the cut leaves the previous value
/// standing (`values.rs` doc-comment).
fn apply_one(
    style: ComputedStyle,
    parent: Option<&ComputedStyle>,
    declaration: &Declaration,
) -> ComputedStyle {
    apply_declaration(style, declaration, parent).unwrap_or(style)
}

fn apply_inline_block(
    style: ComputedStyle,
    parent: Option<&ComputedStyle>,
    node: NodeRef<'_>,
    sheets: &StyleSheetSet,
) -> ComputedStyle {
    let Some(block) = sheets.inline_of(node.id()) else {
        return style;
    };
    apply_block(style, parent, block)
}
