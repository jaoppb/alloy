//! [`CssError`] — the **one** typed error for this port (`ADR-0011` item 4).
//!
//! `thiserror`, not a hand-written `Display`: the manual carve-out of
//! `ADR-0015` applies only to `core/engine`; this crate follows `core/dom`
//! (correction at the top of the v0.5 plan). Every variant carries a
//! [`CssStage`] — the pipeline stage that failed — and an optional
//! [`SourceSpan`], the `ADR-0011:93-95` location metadata for this port.

use core::fmt;

use crate::domain::dom_snapshot::SnapshotId;

/// Which stage of the style pipeline raised an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CssStage {
    /// Tokenizing or parsing a stylesheet (B1).
    Parse,
    /// Matching a selector against the DOM snapshot (B1).
    Selector,
    /// Resolving the cascade to computed values.
    Cascade,
    /// Turning computed values into boxes with geometry.
    Layout,
    /// Measuring a text run.
    Measure,
}

impl CssStage {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Selector => "selector",
            Self::Cascade => "cascade",
            Self::Layout => "layout",
            Self::Measure => "measure",
        }
    }
}

impl fmt::Display for CssStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// A line/column location in a stylesheet. Both are 1-based; `0` is "unknown".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    line: u32,
    column: u32,
}

impl SourceSpan {
    #[must_use]
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    #[must_use]
    pub const fn line(self) -> u32 {
        self.line
    }

    #[must_use]
    pub const fn column(self) -> u32 {
        self.column
    }
}

impl fmt::Display for SourceSpan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.line, self.column)
    }
}

/// A failure raised while resolving the cascade, laying out, or measuring text.
///
/// `Eq` as well as `PartialEq` (every field is `Eq`-capable and the `nursery`
/// lint requires it) — matching `core/dom`'s `DomError`.
#[derive(thiserror::Error, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CssError {
    /// A resolver or layout engine referenced a node absent from the snapshot.
    #[error("{stage} stage: {node} is absent from the DOM snapshot")]
    UnknownNode {
        stage: CssStage,
        node: SnapshotId,
        span: Option<SourceSpan>,
    },

    /// The layout engine reached a styled node with no computed style.
    #[error("{stage} stage: {node} has no computed style")]
    MissingComputedStyle {
        stage: CssStage,
        node: SnapshotId,
        span: Option<SourceSpan>,
    },

    /// The input names something this placeholder adapter does not handle yet.
    #[error("{stage} stage: {detail}")]
    Unsupported {
        stage: CssStage,
        detail: String,
        span: Option<SourceSpan>,
    },
}

impl CssError {
    #[must_use]
    pub const fn unknown_node(stage: CssStage, node: SnapshotId) -> Self {
        Self::UnknownNode {
            stage,
            node,
            span: None,
        }
    }

    #[must_use]
    pub const fn missing_computed_style(stage: CssStage, node: SnapshotId) -> Self {
        Self::MissingComputedStyle {
            stage,
            node,
            span: None,
        }
    }

    #[must_use]
    pub fn unsupported(stage: CssStage, detail: impl Into<String>) -> Self {
        Self::Unsupported {
            stage,
            detail: detail.into(),
            span: None,
        }
    }

    /// The stage this error was raised at.
    #[must_use]
    pub const fn stage(&self) -> CssStage {
        match self {
            Self::UnknownNode { stage, .. }
            | Self::MissingComputedStyle { stage, .. }
            | Self::Unsupported { stage, .. } => *stage,
        }
    }

    /// The same error with `span` attached as its location.
    #[must_use]
    pub const fn with_span(mut self, span: SourceSpan) -> Self {
        match &mut self {
            Self::UnknownNode { span: slot, .. }
            | Self::MissingComputedStyle { span: slot, .. }
            | Self::Unsupported { span: slot, .. } => *slot = Some(span),
        }
        self
    }

    /// This error's location, if one was attached.
    #[must_use]
    pub const fn span(&self) -> Option<SourceSpan> {
        match self {
            Self::UnknownNode { span, .. }
            | Self::MissingComputedStyle { span, .. }
            | Self::Unsupported { span, .. } => *span,
        }
    }
}
