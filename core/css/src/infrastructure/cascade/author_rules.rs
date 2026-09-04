//! [`apply_author_rules`] — the matched rules of a [`StyleSheetSet`], applied to
//! one node in cascade order (the B1 deliverable of `plano:430-431`).
//!
//! The sort key is `(origin precedence, specificity, source order)` and it is
//! **total**: two distinct rules can never tie, because no two occupy the same
//! index. That is what keeps the 100-run determinism check of `PRD-007:100`
//! green — there is no residual ordering for a hash map or an unstable sort to
//! decide.
//!
//! Two things B1 deliberately does not do, both B2's (`plano:435-443`):
//! `!important` is preserved on the [`Declaration`] but does not yet win, and
//! the three origins order by [`Origin::precedence`] alone. What B1 does own is
//! the last step of the cascade, which no later phase changes: the node's
//! `style=` block is applied after every rule (CSS Cascade L4 §6.4.3).

use crate::application::matching::strongest_match;
use crate::domain::computed::style::ComputedStyle;
use crate::domain::declaration::{Declaration, DeclarationBlock};
use crate::domain::dom_snapshot::{DomSnapshot, NodeRef};
use crate::domain::specificity::Specificity;
use crate::domain::stylesheet_set::{Origin, StyleRule, StyleSheetSet};
use crate::infrastructure::cascade::values::apply_declaration;

/// One rule that selected the node, with the key it is ordered by.
struct MatchedRule<'sheets> {
    precedence: u8,
    specificity: Specificity,
    order: usize,
    declarations: &'sheets DeclarationBlock,
}

impl MatchedRule<'_> {
    /// `(origin, specificity, source order)` — CSS Cascade L4 §6.4, steps 3, 5
    /// and 6, in that order.
    const fn sort_key(&self) -> (u8, Specificity, usize) {
        (self.precedence, self.specificity, self.order)
    }
}

/// `base` with every matching rule applied weakest-first, then the node's
/// `style=` block.
#[must_use]
pub(crate) fn apply_author_rules(
    base: ComputedStyle,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
    sheets: &StyleSheetSet,
) -> ComputedStyle {
    let matched = matched_rules(node, snapshot, sheets);
    let cascaded = matched
        .iter()
        .fold(base, |style, rule| apply_block(style, rule.declarations));
    apply_inline_block(cascaded, node, sheets)
}

/// Every rule of `sheets` that selects `node`, in cascade order.
fn matched_rules<'sheets>(
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
    sheets: &'sheets StyleSheetSet,
) -> Vec<MatchedRule<'sheets>> {
    let mut matched: Vec<MatchedRule<'sheets>> = sheets
        .rules()
        .enumerate()
        .filter_map(|(order, (origin, rule))| match_rule(order, origin, rule, node, snapshot))
        .collect();
    matched.sort_by_key(MatchedRule::sort_key);
    matched
}

/// The match, or `None` when the rule does not select the node — or still
/// carries a `@media` condition nobody evaluated.
///
/// A resolver receives no viewport (`PRD-007:56-60`, frozen at I3), so skipping
/// is the only safe reading of an unevaluated condition. The producer discharges
/// them first with [`StyleSheetSet::matching_viewport`].
fn match_rule<'sheets>(
    order: usize,
    origin: Origin,
    rule: &'sheets StyleRule,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
) -> Option<MatchedRule<'sheets>> {
    let media = rule.media();
    if !media.is_always() {
        return None;
    }
    let specificity = strongest_match(rule.selectors(), node, snapshot)?;
    Some(MatchedRule {
        precedence: origin.precedence(),
        specificity,
        order,
        declarations: rule.declarations(),
    })
}

fn apply_block(style: ComputedStyle, block: &DeclarationBlock) -> ComputedStyle {
    block.iter().fold(style, apply_one)
}

/// A declaration whose value is outside the cut leaves the previous value
/// standing (`values.rs` doc-comment).
fn apply_one(style: ComputedStyle, declaration: &Declaration) -> ComputedStyle {
    apply_declaration(style, declaration).unwrap_or(style)
}

fn apply_inline_block(
    style: ComputedStyle,
    node: NodeRef<'_>,
    sheets: &StyleSheetSet,
) -> ComputedStyle {
    let Some(block) = sheets.inline_of(node.id()) else {
        return style;
    };
    apply_block(style, block)
}
