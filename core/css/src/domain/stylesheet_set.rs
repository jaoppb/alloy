//! [`StyleSheetSet`] — parsed, ordered rules with an [`Origin`]
//! (`PRD-007:37-38`).
//!
//! B0 held a rule as selector *text* plus raw string declarations and said the
//! shape would survive B1. It has: the aggregate is still "ordered rules tagged
//! with an origin". What changed is what a rule carries — a parsed
//! [`SelectorList`], a [`DeclarationBlock`] of validated [`Declaration`]s, and
//! the [`MediaQuery`] gating it — plus two additions the document-level parser
//! needs: the per-node `style=` blocks ([`InlineStyles`]) and the
//! [`ParseNotes`] recording everything the parser recovered from.
//! [`crate::PORT_SCHEMA_VERSION`] is bumped to 2 for exactly this
//! (`ADR-0011` item 3).

use core::fmt;

use crate::domain::declaration::DeclarationBlock;
use crate::domain::dom_snapshot::SnapshotId;
use crate::domain::media::MediaQuery;
use crate::domain::parse_notes::{ParseNote, ParseNotes};
use crate::domain::selector::SelectorList;
use crate::domain::viewport::ViewportConstraints;

/// Which stylesheet a rule came from. The cascade orders origins
/// UA < User < Author (`PRD-007:38`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Origin {
    /// The user-agent stylesheet.
    UserAgent,
    /// The user's stylesheet.
    User,
    /// The document author's stylesheets and `style=` attributes.
    Author,
}

impl Origin {
    /// The cascade precedence of this origin: lower sorts first (weaker).
    #[must_use]
    pub const fn precedence(self) -> u8 {
        match self {
            Self::UserAgent => 0,
            Self::User => 1,
            Self::Author => 2,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::UserAgent => "user-agent",
            Self::User => "user",
            Self::Author => "author",
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// One rule: the selectors that choose its subjects, the declarations it
/// applies, and the `@media` conditions gating it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StyleRule {
    selectors: SelectorList,
    declarations: DeclarationBlock,
    media: MediaQuery,
}

impl StyleRule {
    /// An unconditional rule — one written outside every `@media` block.
    #[must_use]
    pub const fn new(selectors: SelectorList, declarations: DeclarationBlock) -> Self {
        Self {
            selectors,
            declarations,
            media: MediaQuery::always(),
        }
    }

    /// The same rule, gated on `media`.
    #[must_use]
    pub fn with_media(mut self, media: MediaQuery) -> Self {
        self.media = media;
        self
    }

    #[must_use]
    pub const fn selectors(&self) -> &SelectorList {
        &self.selectors
    }

    #[must_use]
    pub const fn declarations(&self) -> &DeclarationBlock {
        &self.declarations
    }

    #[must_use]
    pub const fn media(&self) -> &MediaQuery {
        &self.media
    }
}

impl fmt::Display for StyleRule {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {{ {} }}", self.selectors, self.declarations)
    }
}

/// A rule tagged with the origin it came from.
#[derive(Clone, Debug, PartialEq)]
struct OriginRule {
    origin: Origin,
    rule: StyleRule,
}

/// The `style=` block of each element that has one, keyed by the snapshot node
/// it belongs to. A first-class collection — no public `Vec`.
///
/// Inline declarations are author-origin and outrank every author rule
/// (CSS Cascade L4 §6.4.3), which is why they travel beside the rule list
/// rather than inside it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InlineStyles {
    blocks: Vec<(SnapshotId, DeclarationBlock)>,
}

impl InlineStyles {
    #[must_use]
    pub const fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub fn push(&mut self, node: SnapshotId, block: DeclarationBlock) {
        self.blocks.push((node, block));
    }

    /// The block for `node`, or `None`.
    #[must_use]
    pub fn get(&self, node: SnapshotId) -> Option<&DeclarationBlock> {
        self.blocks
            .iter()
            .find(|(candidate, _)| *candidate == node)
            .map(|(_, block)| block)
    }

    pub fn iter(&self) -> impl Iterator<Item = (SnapshotId, &DeclarationBlock)> + '_ {
        self.blocks.iter().map(|(node, block)| (*node, block))
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.blocks.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// Every rule that could apply to a document, in cascade order.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct StyleSheetSet {
    rules: Vec<OriginRule>,
    inline: InlineStyles,
    notes: ParseNotes,
}

impl StyleSheetSet {
    /// An empty set — no rule, no inline block, no note.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rules: Vec::new(),
            inline: InlineStyles::new(),
            notes: ParseNotes::new(),
        }
    }

    /// Appends a rule for `origin`.
    pub fn push_rule(&mut self, origin: Origin, rule: StyleRule) {
        self.rules.push(OriginRule { origin, rule });
    }

    /// Records the `style=` block of one element.
    pub fn push_inline(&mut self, node: SnapshotId, block: DeclarationBlock) {
        self.inline.push(node, block);
    }

    /// Records something the parser recovered from.
    pub fn push_note(&mut self, note: ParseNote) {
        self.notes.push(note);
    }

    /// Appends every rule, inline block and note of `other`, preserving both
    /// source orders — how a document's several `<style>` elements become one
    /// set.
    pub fn absorb(&mut self, other: Self) {
        self.rules.extend(other.rules);
        for (node, block) in other.inline.iter() {
            self.inline.push(node, block.clone());
        }
        self.notes.absorb(other.notes);
    }

    pub fn rules(&self) -> impl Iterator<Item = (Origin, &StyleRule)> + '_ {
        self.rules.iter().map(|entry| (entry.origin, &entry.rule))
    }

    /// The `style=` block of `node`, or `None`.
    #[must_use]
    pub fn inline_of(&self, node: SnapshotId) -> Option<&DeclarationBlock> {
        self.inline.get(node)
    }

    #[must_use]
    pub const fn inline(&self) -> &InlineStyles {
        &self.inline
    }

    #[must_use]
    pub const fn notes(&self) -> &ParseNotes {
        &self.notes
    }

    /// The same set with every `@media` rule resolved against `constraints`:
    /// a rule whose conditions hold survives **unconditionally**, a rule whose
    /// conditions fail is dropped.
    ///
    /// This is the producer-side step `PRD-007:56-60` forces —
    /// [`crate::CascadeResolver::resolve`] receives no viewport, so a resolver
    /// cannot evaluate a media query and skips any rule still carrying one.
    #[must_use]
    pub fn matching_viewport(&self, constraints: &ViewportConstraints) -> Self {
        let rules = self
            .rules
            .iter()
            .filter(|entry| entry.rule.media.matches(constraints))
            .map(unconditional);
        Self {
            rules: rules.collect(),
            inline: self.inline.clone(),
            notes: self.notes.clone(),
        }
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.rules.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// The same origin-tagged rule with its media conditions discharged.
fn unconditional(entry: &OriginRule) -> OriginRule {
    OriginRule {
        origin: entry.origin,
        rule: StyleRule::new(
            entry.rule.selectors.clone(),
            entry.rule.declarations.clone(),
        ),
    }
}
