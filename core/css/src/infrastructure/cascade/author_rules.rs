//! [`apply_author_rules`] — the matched declarations of a [`StyleSheetSet`],
//! applied to one node in cascade order (`plano:430-431`, `:435-443`).
//!
//! The unit that gets sorted is the **declaration**, not the rule: a single
//! rule can mix an `!important` declaration with a normal one
//! (`p { color: red !important; margin: 4px }`), and CSS Cascade L4 §4.2
//! ranks those two independently. Flattening to `(precedence, specificity,
//! source order, position in block)` before sorting is what keeps that case
//! correct without a second sort.
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
//!
//! The one cascade-ordered list is then folded in three passes, each over the
//! declarations it owns, every pass keeping cascade order:
//!
//! 1. **custom properties** (`--*`) — the element's [`CustomPropertiesMap`],
//!    inherited from its parent's and with its own `var()` references
//!    substituted (CSS Variables L1 §2);
//! 2. **the writing context** — `writing-mode` and `direction`, so the element's
//!    final context is known before any flow-relative property maps to a
//!    physical one (CSS Logical L1 §4: a logical property and its physical
//!    counterpart share one computed value, last in cascade order wins);
//! 3. **everything else**, with `var()` substituted from pass 1's map. A
//!    declaration whose substitution fails, or whose substituted value does
//!    not parse, is invalid at computed-value time and computes to `unset`
//!    (CSS Variables L1 §3) — never to the previous cascaded value.
//!
//! Finally the font-relative text lengths are made absolute against the
//! element's own computed font size (CSS 2.1 §10.8.1, CSS Text L3 §8).

use std::rc::Rc;

use graphics::Au;

use crate::application::matching::strongest_match;
use crate::domain::computed::style::{ComputedStyle, INITIAL_FONT_SIZE};
use crate::domain::computed::variables::{CustomPropertiesMap, VariableName};
use crate::domain::declaration::{Declaration, DeclarationBlock};
use crate::domain::dom_snapshot::{DomSnapshot, NodeRef};
use crate::domain::specificity::Specificity;
use crate::domain::stylesheet_set::{Origin, StyleRule, StyleSheetSet};
use crate::infrastructure::cascade::logical_values::sets_writing_context;
use crate::infrastructure::cascade::values::{
    apply_declaration, apply_declaration_value, unset_property,
};
use crate::infrastructure::cascade::variable_values::{
    cascade_custom_properties, references_variables, resolve_declaration_value,
};

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

/// What an element hands down to its children beyond its [`ComputedStyle`]:
/// its computed custom properties and its computed font size. Neither fits in
/// the `Copy` style aggregate — the map is open-ended, and the font size is
/// stored there as authored (layout resolves it) — so the cascade keeps them
/// in a side table, one per node.
#[derive(Clone, Debug)]
pub(crate) struct InheritedContext {
    variables: Rc<CustomPropertiesMap>,
    font_size: Au,
}

impl InheritedContext {
    /// What the root inherits: no custom properties, and the initial `16px`
    /// its `em` resolves against.
    #[must_use]
    pub(crate) fn root() -> Self {
        Self {
            variables: Rc::new(CustomPropertiesMap::new()),
            font_size: INITIAL_FONT_SIZE,
        }
    }
}

/// One node's cascade result: its style, and the context its children inherit.
pub(crate) struct CascadedStyle {
    style: ComputedStyle,
    context: InheritedContext,
}

impl CascadedStyle {
    /// The style and the inherited context, apart.
    #[must_use]
    pub(crate) fn into_parts(self) -> (ComputedStyle, InheritedContext) {
        (self.style, self.context)
    }
}

/// Which of the three passes (module doc) folds a declaration.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CascadePass {
    CustomProperties,
    WritingContext,
    Remaining,
}

impl CascadePass {
    fn of(declaration: &Declaration) -> Self {
        let property = declaration.property().as_str();
        if VariableName::names_custom_property(property) {
            return Self::CustomProperties;
        }
        if sets_writing_context(property) {
            return Self::WritingContext;
        }
        Self::Remaining
    }
}

/// What one declaration of pass 2 or 3 is applied with: the parent's style
/// (for `inherit`, `unset` and `bolder`) and the element's custom properties.
struct DeclarationScope<'element> {
    parent: Option<&'element ComputedStyle>,
    variables: &'element CustomPropertiesMap,
}

impl DeclarationScope<'_> {
    /// `style` with `declaration` applied. A value with no `var()` that falls
    /// outside the cut leaves the previous value standing (`values.rs`
    /// doc-comment) — the parse-time drop of CSS Syntax L3.
    fn apply(&self, style: ComputedStyle, declaration: &Declaration) -> ComputedStyle {
        if references_variables(declaration.value()) {
            return self.apply_substituted(style, declaration);
        }
        apply_declaration(style, declaration, self.parent).unwrap_or(style)
    }

    /// A `var()`-bearing declaration: substituted, then applied, or `unset`
    /// when either step fails (CSS Variables L1 §3, invalid at computed-value
    /// time).
    fn apply_substituted(&self, style: ComputedStyle, declaration: &Declaration) -> ComputedStyle {
        let property = declaration.property().as_str();
        let substituted = resolve_declaration_value(declaration.value(), self.variables)
            .inspect_err(|error| {
                tracing::debug!(property, %error, "declaration is invalid at computed-value time");
            })
            .ok();
        substituted
            .and_then(|value| apply_declaration_value(style, self.parent, property, &value))
            .unwrap_or_else(|| unset_property(style, self.parent, property))
    }

    fn fold_pass(
        &self,
        style: ComputedStyle,
        declarations: &[&Declaration],
        pass: CascadePass,
    ) -> ComputedStyle {
        declarations
            .iter()
            .filter(|declaration| CascadePass::of(declaration) == pass)
            .fold(style, |folded, declaration| self.apply(folded, declaration))
    }
}

/// `base` with every matching declaration applied weakest-first, then the
/// node's `style=` block, in the three passes of the module doc.
#[must_use]
pub(crate) fn apply_author_rules(
    base: ComputedStyle,
    parent: Option<&ComputedStyle>,
    inherited: &InheritedContext,
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
    sheets: &StyleSheetSet,
) -> CascadedStyle {
    let declarations = cascade_ordered_declarations(node, snapshot, sheets);
    let variables = cascade_custom_properties(&inherited.variables, &declarations);
    let scope = DeclarationScope {
        parent,
        variables: &variables,
    };
    let oriented = scope.fold_pass(base, &declarations, CascadePass::WritingContext);
    let cascaded = scope.fold_pass(oriented, &declarations, CascadePass::Remaining);
    let font_size = cascaded.computed_font_size(inherited.font_size);
    CascadedStyle {
        style: cascaded.with_text_advance(cascaded.text_advance().absolutized(font_size)),
        context: InheritedContext {
            variables,
            font_size,
        },
    }
}

/// Every declaration that applies to `node`, weakest first: the matched rule
/// declarations in cascade order, then the node's `style=` block.
fn cascade_ordered_declarations<'sheets>(
    node: NodeRef<'_>,
    snapshot: &DomSnapshot,
    sheets: &'sheets StyleSheetSet,
) -> Vec<&'sheets Declaration> {
    let matched = matched_declarations(node, snapshot, sheets);
    let inline = sheets
        .inline_of(node.id())
        .into_iter()
        .flat_map(DeclarationBlock::iter);
    matched
        .into_iter()
        .map(|matched| matched.declaration)
        .chain(inline)
        .collect()
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
