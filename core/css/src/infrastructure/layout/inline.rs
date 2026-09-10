//! The inline formatting context (v0.5 B4, step 2).
//!
//! A run of inline-level boxes becomes a stack of **line boxes**: the text is
//! white-space-processed (CSS Text L3 §4.1.1), segmented into words and break
//! opportunities, measured **through the [`crate::TextMeasurer`] port** — no
//! font type is named here — filled greedily into lines, and each line is then
//! aligned by `text-align` (CSS Text L3 §7.3).
//!
//! Two simplifications are declared in `core/css/tests/data/MANIFEST.md`:
//! an inline box's fragment is the bounding box of its pieces (so an inline
//! that spans two lines gets one rectangle, not two), and inline boxes carry no
//! border or padding of their own.

use graphics::{Au, Point, Rect};

use crate::domain::computed::inline_style::{TextAlign, WhiteSpace};
use crate::domain::dom_snapshot::{ChildIds, SnapshotId};
use crate::domain::error::CssError;
use crate::domain::layout_box_tree::BoxEdges;
use crate::domain::styled_tree::StyledNode;
use crate::domain::text::TextMetrics;
use crate::infrastructure::layout::box_model;
use crate::infrastructure::layout::context::{ContentFlow, LayoutContext};
use crate::infrastructure::layout::fragment::{Fragment, Fragments, rect_at};

/// The one space a collapsed run of white space becomes.
const COLLAPSED_SPACE: &str = " ";

/// What a piece of segmented text is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PieceKind {
    /// Ink: a word, or a whole preserved line under `white-space: pre`.
    Word,
    /// A single space between two words — a soft wrap opportunity.
    Space,
    /// A `\n` under `white-space: pre` — a mandatory break.
    Break,
}

/// One measured piece of an inline run.
#[derive(Clone, Copy)]
struct Piece {
    node: SnapshotId,
    kind: PieceKind,
    metrics: TextMetrics,
    white_space: WhiteSpace,
}

/// One line box: the pieces on it and the extents that position them.
struct Line {
    pieces: Vec<Piece>,
    width: Au,
    ascent: Au,
    descent: Au,
}

impl Line {
    const fn new() -> Self {
        Self {
            pieces: Vec::new(),
            width: Au::ZERO,
            ascent: Au::ZERO,
            descent: Au::ZERO,
        }
    }

    fn push(&mut self, piece: Piece) {
        let metrics = piece.metrics;
        self.width = self.width.saturating_add(metrics.width());
        self.ascent = self.ascent.larger(metrics.baseline());
        self.descent = self.descent.larger(metrics.descent());
        self.pieces.push(piece);
    }

    const fn height(&self) -> Au {
        self.ascent.saturating_add(self.descent)
    }

    const fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// How many soft wrap opportunities sit between this line's words — the
    /// gaps `text-align: justify` widens.
    fn gap_count(&self) -> usize {
        self.pieces
            .iter()
            .filter(|piece| piece.kind == PieceKind::Space)
            .count()
    }
}

/// Lays a run of inline-level nodes out inside a content box `content_width`
/// wide.
pub fn layout(
    context: &LayoutContext<'_>,
    items: &[SnapshotId],
    content_width: Au,
    font_size: Au,
    align: TextAlign,
) -> Result<ContentFlow, CssError> {
    let mut pieces = Vec::new();
    for item in items {
        collect(context, *item, font_size, &mut pieces)?;
    }
    let lines = fill_lines(&pieces, content_width);
    let placements = place(&lines, content_width, align);
    let fragments = emit(context, items, &placements)?;
    Ok(ContentFlow::new(placements.height, fragments))
}

// ---- collection and white-space processing --------------------------------

/// Walks one inline-level node, appending its measured pieces in document
/// order. An inline box contributes nothing itself; its descendants do.
fn collect(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    parent_font_size: Au,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    let styled = context.node(node_id)?;
    let style = styled.style();
    let font_size = box_model::font_size_of(style, parent_font_size);
    let Some(run) = styled.text() else {
        return collect_children(context, styled, font_size, pieces);
    };
    let white_space = style.white_space();
    append_text(
        context,
        node_id,
        Setting::new(font_size, white_space),
        run.as_str(),
        pieces,
    )
}

fn collect_children(
    context: &LayoutContext<'_>,
    styled: &StyledNode,
    font_size: Au,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    for child in styled.children().iter() {
        collect(context, child, font_size, pieces)?;
    }
    Ok(())
}

/// The two things segmentation needs to know about the node the text came from.
#[derive(Clone, Copy)]
struct Setting {
    font_size: Au,
    white_space: WhiteSpace,
}

impl Setting {
    const fn new(font_size: Au, white_space: WhiteSpace) -> Self {
        Self {
            font_size,
            white_space,
        }
    }
}

fn append_text(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    text: &str,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    if setting.white_space.collapses_spaces() {
        return append_collapsed(context, node_id, setting, text, pieces);
    }
    append_preserved(context, node_id, setting, text, pieces)
}

/// `white-space: normal` / `nowrap`: every run of white space becomes one
/// space, and a hyphen inside a word is a further break opportunity
/// (a simplified UAX #14).
fn append_collapsed(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    text: &str,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    let leading = text.starts_with(char::is_whitespace);
    let trailing = text.ends_with(char::is_whitespace);
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return append_border_space(context, node_id, setting, leading, pieces);
    }
    append_space_when(context, node_id, setting, leading, pieces)?;
    append_words(context, node_id, setting, &words, pieces)?;
    append_space_when(context, node_id, setting, trailing, pieces)
}

/// A run made only of white space still separates its neighbours.
fn append_border_space(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    any: bool,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    append_space_when(context, node_id, setting, any, pieces)
}

fn append_words(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    words: &[&str],
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    for (index, word) in words.iter().enumerate() {
        append_separator(context, node_id, setting, index, pieces)?;
        append_word_parts(context, node_id, setting, word, pieces)?;
    }
    Ok(())
}

fn append_separator(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    index: usize,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    append_space_when(context, node_id, setting, index > 0, pieces)
}

/// A hyphenated word breaks after each hyphen, with the hyphen staying on the
/// left-hand part — the one UAX #14 rule this cut implements beyond the space.
fn append_word_parts(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    word: &str,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    for part in hyphen_parts(word) {
        let metrics = context.measure(&part, setting.font_size)?;
        pieces.push(piece(node_id, PieceKind::Word, metrics, setting));
    }
    Ok(())
}

/// `well-known` → `["well-", "known"]`; a word with no hyphen is one part.
fn hyphen_parts(word: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    for character in word.chars() {
        current.push(character);
        flush_after_hyphen(&mut parts, &mut current, character);
    }
    push_remainder(&mut parts, current);
    parts
}

fn flush_after_hyphen(parts: &mut Vec<String>, current: &mut String, character: char) {
    if character != '-' {
        return;
    }
    parts.push(core::mem::take(current));
}

fn push_remainder(parts: &mut Vec<String>, current: String) {
    if current.is_empty() {
        return;
    }
    parts.push(current);
}

fn append_space_when(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    wanted: bool,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    if !wanted {
        return Ok(());
    }
    let metrics = context.measure(COLLAPSED_SPACE, setting.font_size)?;
    pieces.push(piece(node_id, PieceKind::Space, metrics, setting));
    Ok(())
}

/// `white-space: pre`: nothing collapses, and only a `\n` breaks a line.
fn append_preserved(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    text: &str,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    for (index, line) in text.split('\n').enumerate() {
        append_forced_break(context, node_id, setting, index, pieces)?;
        append_preserved_line(context, node_id, setting, line, pieces)?;
    }
    Ok(())
}

fn append_forced_break(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    index: usize,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    if index == 0 {
        return Ok(());
    }
    let metrics = context.measure("", setting.font_size)?;
    pieces.push(piece(node_id, PieceKind::Break, metrics, setting));
    Ok(())
}

fn append_preserved_line(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    setting: Setting,
    line: &str,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    if line.is_empty() {
        return Ok(());
    }
    let metrics = context.measure(line, setting.font_size)?;
    pieces.push(piece(node_id, PieceKind::Word, metrics, setting));
    Ok(())
}

const fn piece(node: SnapshotId, kind: PieceKind, metrics: TextMetrics, setting: Setting) -> Piece {
    Piece {
        node,
        kind,
        metrics,
        white_space: setting.white_space,
    }
}

// ---- line filling ---------------------------------------------------------

/// Greedy line breaking: a word that does not fit opens a new line, and the
/// space that would have preceded it is dropped. A word wider than the whole
/// line overflows rather than being split — there is no hyphenation here.
fn fill_lines(pieces: &[Piece], limit: Au) -> Vec<Line> {
    let mut filler = LineFiller::new(limit);
    for piece in pieces {
        filler.absorb(*piece);
    }
    filler.finish()
}

struct LineFiller {
    lines: Vec<Line>,
    current: Line,
    pending_space: Option<Piece>,
    limit: Au,
}

impl LineFiller {
    const fn new(limit: Au) -> Self {
        Self {
            lines: Vec::new(),
            current: Line::new(),
            pending_space: None,
            limit,
        }
    }

    fn absorb(&mut self, piece: Piece) {
        match piece.kind {
            PieceKind::Break => self.force_break(piece),
            PieceKind::Space => self.hold_space(piece),
            PieceKind::Word => self.place_word(piece),
        }
    }

    fn force_break(&mut self, piece: Piece) {
        self.pending_space = None;
        self.absorb_empty_line(piece);
        self.close();
    }

    /// A `\n` on an otherwise empty line still occupies one line's height.
    fn absorb_empty_line(&mut self, piece: Piece) {
        if !self.current.is_empty() {
            return;
        }
        self.current.push(piece);
    }

    const fn hold_space(&mut self, piece: Piece) {
        if self.current.is_empty() {
            return;
        }
        self.pending_space = Some(piece);
    }

    fn place_word(&mut self, piece: Piece) {
        if self.overflows(piece) {
            self.pending_space = None;
            self.close();
        }
        self.flush_pending_space();
        self.current.push(piece);
    }

    fn overflows(&self, piece: Piece) -> bool {
        if self.current.is_empty() || !piece.white_space.allows_soft_wrap() {
            return false;
        }
        let metrics = piece.metrics;
        let space = self.pending_width();
        let candidate = self
            .current
            .width
            .saturating_add(space)
            .saturating_add(metrics.width());
        candidate > self.limit
    }

    fn pending_width(&self) -> Au {
        self.pending_space
            .map_or(Au::ZERO, |space| space.metrics.width())
    }

    fn flush_pending_space(&mut self) {
        let Some(space) = self.pending_space.take() else {
            return;
        };
        self.current.push(space);
    }

    fn close(&mut self) {
        let finished = core::mem::replace(&mut self.current, Line::new());
        self.lines.push(finished);
    }

    fn finish(mut self) -> Vec<Line> {
        if !self.current.is_empty() {
            self.close();
        }
        self.lines
    }
}

// ---- alignment and placement ---------------------------------------------

/// Where every piece ended up, and how tall the whole run is.
struct Placements {
    height: Au,
    rects: Vec<(SnapshotId, Rect)>,
}

fn place(lines: &[Line], content_width: Au, align: TextAlign) -> Placements {
    let mut cursor = Au::ZERO;
    let mut rects = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let last = index.saturating_add(1) == lines.len();
        place_line(
            line,
            LinePlacement::new(content_width, align, cursor, last),
            &mut rects,
        );
        cursor = cursor.saturating_add(line.height());
    }
    Placements {
        height: cursor,
        rects,
    }
}

/// Everything one line needs to know about where it sits and how it aligns.
#[derive(Clone, Copy)]
struct LinePlacement {
    content_width: Au,
    align: TextAlign,
    top: Au,
    is_last: bool,
}

impl LinePlacement {
    const fn new(content_width: Au, align: TextAlign, top: Au, is_last: bool) -> Self {
        Self {
            content_width,
            align,
            top,
            is_last,
        }
    }
}

fn place_line(line: &Line, placement: LinePlacement, rects: &mut Vec<(SnapshotId, Rect)>) {
    let mut cursor = start_offset(line, placement);
    let extra = justification(line, placement);
    for (index, piece) in line.pieces.iter().enumerate() {
        let widened = widened_width(*piece, extra, index);
        push_rect(line, *piece, Point::new(cursor, placement.top), rects);
        cursor = cursor.saturating_add(widened);
    }
}

fn push_rect(line: &Line, piece: Piece, origin: Point, rects: &mut Vec<(SnapshotId, Rect)>) {
    let metrics = piece.metrics;
    let top = origin
        .vertical()
        .saturating_add(line.ascent)
        .saturating_sub(metrics.baseline());
    let placed = rect_at(
        Point::new(origin.horizontal(), top),
        metrics.width(),
        metrics.height(),
    );
    rects.push((piece.node, placed));
}

/// The `text-align` offset of a line's first piece. `justify` starts flush
/// left and widens the gaps instead.
const fn start_offset(line: &Line, placement: LinePlacement) -> Au {
    let free = placement.content_width.saturating_sub(line.width);
    if free.is_negative() {
        return Au::ZERO;
    }
    match placement.align {
        TextAlign::Left | TextAlign::Justify => Au::ZERO,
        TextAlign::Right => free,
        TextAlign::Center => half(free),
    }
}

const fn half(value: Au) -> Au {
    Au::from_raw(value.raw() / 2)
}

/// How much every inter-word gap grows under `text-align: justify`. The last
/// line of a block is never justified (CSS Text L3 §7.3).
fn justification(line: &Line, placement: LinePlacement) -> Extra {
    if placement.align != TextAlign::Justify || placement.is_last {
        return Extra::NONE;
    }
    let free = placement.content_width.saturating_sub(line.width);
    if free.is_negative() || free.is_zero() {
        return Extra::NONE;
    }
    Extra::spread(free, line.gap_count())
}

/// A quantity split evenly over the gaps of one line, remainder first, so the
/// result is exact and deterministic.
#[derive(Clone, Copy)]
struct Extra {
    per_gap: i32,
    remainder: i32,
}

impl Extra {
    const NONE: Self = Self {
        per_gap: 0,
        remainder: 0,
    };

    fn spread(free: Au, gaps: usize) -> Self {
        let Ok(count) = i32::try_from(gaps) else {
            return Self::NONE;
        };
        if count == 0 {
            return Self::NONE;
        }
        let raw = free.raw();
        let per_gap = raw.checked_div(count).unwrap_or(0);
        let remainder = raw.checked_rem(count).unwrap_or(0);
        Self { per_gap, remainder }
    }

    /// The extra given to the gap at `index` among the ones already widened.
    fn share(self, widened: i32) -> Au {
        let bonus = i32::from(widened < self.remainder);
        Au::from_raw(self.per_gap.saturating_add(bonus))
    }
}

/// A gap's advance under justification is its own width plus its share; every
/// other piece advances by its own width.
fn widened_width(piece: Piece, extra: Extra, index: usize) -> Au {
    let metrics = piece.metrics;
    if piece.kind != PieceKind::Space {
        return metrics.width();
    }
    let Ok(position) = i32::try_from(index) else {
        return metrics.width();
    };
    metrics.width().saturating_add(extra.share(position))
}

// ---- fragment emission ----------------------------------------------------

/// One fragment per inline node, in document order: an inline box first, then
/// the boxes it contains.
fn emit(
    context: &LayoutContext<'_>,
    items: &[SnapshotId],
    placements: &Placements,
) -> Result<Fragments, CssError> {
    let mut fragments = Fragments::new();
    for item in items {
        emit_node(context, *item, placements, &mut fragments)?;
    }
    Ok(fragments)
}

fn emit_node(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    placements: &Placements,
    fragments: &mut Fragments,
) -> Result<(), CssError> {
    let styled = context.node(node_id)?;
    let Some(bounds) = union_of(context, node_id, placements)? else {
        return Ok(());
    };
    fragments.push(Fragment::new(
        node_id,
        bounds,
        BoxEdges::ZERO,
        styled.intrinsic_size(),
        ChildIds::from_ids(styled.children().iter()),
    ));
    emit_children(context, styled, placements, fragments)
}

fn emit_children(
    context: &LayoutContext<'_>,
    styled: &StyledNode,
    placements: &Placements,
    fragments: &mut Fragments,
) -> Result<(), CssError> {
    for child in styled.children().iter() {
        emit_node(context, child, placements, fragments)?;
    }
    Ok(())
}

/// The bounding rectangle of everything `node_id`'s subtree put on a line.
fn union_of(
    context: &LayoutContext<'_>,
    node_id: SnapshotId,
    placements: &Placements,
) -> Result<Option<Rect>, CssError> {
    let mut bounds = own_rect(node_id, placements);
    let styled = context.node(node_id)?;
    for child in styled.children().iter() {
        let child_bounds = union_of(context, child, placements)?;
        bounds = merged(bounds, child_bounds);
    }
    Ok(bounds)
}

fn own_rect(node_id: SnapshotId, placements: &Placements) -> Option<Rect> {
    placements
        .rects
        .iter()
        .filter(|(id, _)| *id == node_id)
        .map(|(_, rect)| *rect)
        .reduce(union)
}

fn merged(left: Option<Rect>, right: Option<Rect>) -> Option<Rect> {
    let Some(first) = left else {
        return right;
    };
    let Some(second) = right else {
        return Some(first);
    };
    Some(union(first, second))
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
