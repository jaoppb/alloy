//! Fragment emission for one inline run: placed pieces become fragments.
//!
//! An inline box or a text node gets **one fragment per line box it touches**
//! (#134): the bounding rectangle of what its subtree put on that line, never
//! one rectangle spanning several lines. A text node's fragment also carries
//! the text it holds on that line, so a painter draws exactly what layout
//! placed. An atomic inline keeps its own laid-out fragments, moved onto its
//! slot on the line.

use std::collections::BTreeMap;

use graphics::{Point, Rect};

use super::{Atoms, Placements};
use crate::application::ports::TextMeasurer;
use crate::domain::dom_snapshot::{ChildIds, SnapshotId};
use crate::domain::error::CssError;
use crate::domain::layout_box_tree::{BoxEdges, FragmentText};
use crate::domain::styled_tree::StyledNode;
use crate::infrastructure::layout::context::LayoutContext;
use crate::infrastructure::layout::fragment::{Fragment, Fragments, rect_at};

/// Turns placed pieces into fragments, in document order: an inline box's
/// fragments (one per line, top to bottom) first, then the boxes it contains.
pub(super) struct Emitter<'run, 'tree, M> {
    context: &'run LayoutContext<'tree, M>,
    placements: &'run Placements,
    atoms: Atoms,
    fragments: Fragments,
}

impl<'run, 'tree, M: TextMeasurer> Emitter<'run, 'tree, M> {
    pub(super) const fn new(
        context: &'run LayoutContext<'tree, M>,
        placements: &'run Placements,
        atoms: Atoms,
    ) -> Self {
        Self {
            context,
            placements,
            atoms,
            fragments: Fragments::new(),
        }
    }

    pub(super) fn emit_node(&mut self, node_id: SnapshotId) -> Result<(), CssError> {
        let styled = self.context.node(node_id)?;
        let display = styled.style().display();
        // A block-level descendant left this run when the run split around it
        // (CSS 2.1 §9.2.1.1): it holds no piece here, so its subtree is not
        // walked, once per run, for nothing.
        if !display.is_inline_level() {
            return Ok(());
        }
        if display.is_atomic_inline() {
            self.emit_atom(node_id);
            return Ok(());
        }
        self.emit_box(styled)
    }

    /// An inline box's or a text node's fragment on every line it touches,
    /// then its children's.
    pub(super) fn emit_box(&mut self, styled: &StyledNode) -> Result<(), CssError> {
        let bounds = line_bounds(self.context, styled.node(), self.placements)?;
        for (line, rect) in bounds.into_lines() {
            let fragment = self.line_fragment(styled, line, rect);
            self.fragments.push(fragment);
        }
        for child in styled.children().iter() {
            self.emit_node(child)?;
        }
        Ok(())
    }

    fn line_fragment(&self, styled: &StyledNode, line: usize, rect: Rect) -> Fragment {
        let fragment = Fragment::new(
            styled.node(),
            rect,
            BoxEdges::ZERO,
            styled.intrinsic_size(),
            ChildIds::from_ids(styled.children().iter()),
        );
        let Some(text) = self.text_on(styled, line) else {
            return fragment;
        };
        fragment.with_text(text)
    }

    /// The text a text node put on `line`, or `None` for an element.
    fn text_on(&self, styled: &StyledNode, line: usize) -> Option<FragmentText> {
        styled.text()?;
        let node_id = styled.node();
        let text: String = self
            .placements
            .pieces
            .iter()
            .filter(|placed| placed.node == node_id && placed.line == line)
            .filter_map(|placed| placed.text.as_deref())
            .collect();
        Some(FragmentText::new(text))
    }

    /// The atom's piece occupies its margin box; its fragments are relative to
    /// its border-box origin, one margin in from that.
    fn emit_atom(&mut self, node_id: SnapshotId) {
        let Some(result) = self.atoms.take(node_id) else {
            return;
        };
        let Some(slot) = atom_slot(node_id, self.placements) else {
            return;
        };
        let margin = result.edges().margin();
        let horizontal = slot.min_x().saturating_add(margin.left());
        let vertical = slot.min_y().saturating_add(margin.top());
        self.fragments
            .absorb(result.into_fragments().translated(horizontal, vertical));
    }

    pub(super) fn finish(self) -> Fragments {
        self.fragments
    }
}

/// The one piece an atomic inline is on its line.
fn atom_slot(node_id: SnapshotId, placements: &Placements) -> Option<Rect> {
    placements
        .pieces
        .iter()
        .find(|placed| placed.node == node_id)
        .map(|placed| placed.rect)
}

/// What `node_id`'s whole subtree put on each line, as one bounding rectangle
/// per line.
fn line_bounds<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    placements: &Placements,
) -> Result<LineBounds, CssError> {
    let mut bounds = own_line_bounds(node_id, placements);
    let styled = context.node(node_id)?;
    for child in styled.children().iter() {
        bounds.absorb(child_line_bounds(context, child, placements)?);
    }
    Ok(bounds)
}

/// A child's share of its parent's line bounds. A block-level child left the
/// run when it split around it (CSS 2.1 §9.2.1.1) and holds no piece here, so
/// its subtree is not walked again for every run of the split.
fn child_line_bounds<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    child: SnapshotId,
    placements: &Placements,
) -> Result<LineBounds, CssError> {
    let display = context.node(child)?.style().display();
    if !display.is_inline_level() {
        return Ok(LineBounds::new());
    }
    line_bounds(context, child, placements)
}

fn own_line_bounds(node_id: SnapshotId, placements: &Placements) -> LineBounds {
    let mut bounds = LineBounds::new();
    for placed in placements
        .pieces
        .iter()
        .filter(|placed| placed.node == node_id)
    {
        bounds.add(placed.line, placed.rect);
    }
    bounds
}

/// One bounding rectangle per line index, ordered top to bottom.
struct LineBounds {
    by_line: BTreeMap<usize, Rect>,
}

impl LineBounds {
    const fn new() -> Self {
        Self {
            by_line: BTreeMap::new(),
        }
    }

    fn add(&mut self, line: usize, rect: Rect) {
        let merged = self
            .by_line
            .get(&line)
            .map_or(rect, |existing| union(*existing, rect));
        self.by_line.insert(line, merged);
    }

    fn absorb(&mut self, other: Self) {
        for (line, rect) in other.by_line {
            self.add(line, rect);
        }
    }

    fn into_lines(self) -> impl Iterator<Item = (usize, Rect)> {
        self.by_line.into_iter()
    }
}

fn union(left: Rect, right: Rect) -> Rect {
    let origin = Point::new(
        left.min_x().smaller(right.min_x()),
        left.min_y().smaller(right.min_y()),
    );
    let width = left
        .max_x()
        .larger(right.max_x())
        .saturating_sub(origin.horizontal());
    let height = left
        .max_y()
        .larger(right.max_y())
        .saturating_sub(origin.vertical());
    rect_at(origin, width, height)
}
