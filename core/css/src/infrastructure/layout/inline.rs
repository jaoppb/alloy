//! The inline formatting context (v0.5 B4, step 2).
//!
//! A run of inline-level boxes becomes a stack of **line boxes**: the text is
//! white-space-processed (CSS Text L3 §4.1.1), segmented into words and break
//! opportunities, measured **through the [`crate::TextMeasurer`] port** — no
//! font type is named here — filled greedily into lines, and each line is then
//! aligned by `text-align` (CSS Text L3 §7.3).
//!
//! An `inline-block` descendant is an **atomic inline** (CSS 2.1 §9.2.4): it is
//! laid out as its own block container through
//! [`block::layout_inline_block`], then sits on a line as one unbreakable piece
//! the size of its margin box, its baseline on its bottom margin edge (the
//! CSS 2.1 §10.8.1 rule for an inline-block without in-flow line boxes, applied
//! to every inline-block here). A `display: none` descendant contributes
//! nothing.
//!
//! An inline box or a text node gets one fragment per line it touches, and a
//! text fragment carries the text on its line (`inline/emit.rs`, #134). Inline boxes
//! carry no border or padding of their own (MANIFEST, "Inline formatting
//! simplifications").

use std::collections::BTreeMap;
use std::rc::Rc;

use graphics::{Au, Point, Rect};

use crate::application::ports::TextMeasurer;
use crate::domain::computed::inline_style::{TextAlign, WhiteSpace};
use crate::domain::dom_snapshot::SnapshotId;
use crate::domain::error::CssError;
use crate::domain::styled_tree::StyledNode;
use crate::domain::text::TextMetrics;
use crate::infrastructure::layout::block;
use crate::infrastructure::layout::box_model;
use crate::infrastructure::layout::context::{BlockInput, BlockResult, ContentFlow, LayoutContext};
use crate::infrastructure::layout::fragment::rect_at;

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
    /// An atomic inline (`inline-block`): placed like a word, never split.
    Atomic,
}

/// One measured piece of an inline run, with the text it draws — `None` for
/// an atomic inline and a forced break, which draw none.
#[derive(Clone)]
struct Piece {
    node: SnapshotId,
    kind: PieceKind,
    metrics: TextMetrics,
    white_space: WhiteSpace,
    text: Option<Rc<str>>,
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

/// Lays a run of inline-level nodes out inside the containing block `input`
/// describes — the content box of the block container the run belongs to.
/// `input` is what a child of that container is laid out with: an atomic
/// inline in the run is laid out against it, one level deeper than the
/// container, so `MAX_LAYOUT_DEPTH` still bounds the recursion.
///
/// An item split by block-level descendants contributes only its stretch
/// between them (CSS 2.1 §9.2.1.1, [`window`]); its inline boxes then get
/// fragments only on the lines that stretch fills.
pub fn layout<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    items: &[InlineItem],
    input: BlockInput,
    align: TextAlign,
) -> Result<ContentFlow, CssError> {
    let mut run = Run::new();
    for item in items {
        run.walk = item.walk();
        collect(context, item.node(), input, &mut run)?;
    }
    let placements = place_run(&run.pieces, input.containing_width(), align);
    let mut emitter = Emitter::new(context, &placements, run.atoms);
    for item in items {
        emitter.emit_node(item.node())?;
    }
    Ok(ContentFlow::new(placements.height, emitter.finish()))
}

/// Lays the text a childless node carries itself (a text node blockified by a
/// flex container, an `<input>`'s synthesized label) out as its container's
/// only line content. The node *is* the container here, so its own display —
/// `inline-block` for an `<input>` — is not consulted again, and the font size
/// `input` carries is already the node's own.
pub fn layout_own_text<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    input: BlockInput,
    align: TextAlign,
) -> Result<ContentFlow, CssError> {
    let styled = context.node(node_id)?;
    let mut pieces = Vec::new();
    collect_text(context, styled, input.parent_font_size(), &mut pieces)?;
    let placements = place_run(&pieces, input.containing_width(), align);
    let mut emitter = Emitter::new(context, &placements, Atoms::new());
    emitter.emit_box(styled)?;
    Ok(ContentFlow::new(placements.height, emitter.finish()))
}

fn place_run(pieces: &[Piece], content_width: Au, align: TextAlign) -> Placements {
    let lines = fill_lines(pieces, content_width);
    place(&lines, content_width, align)
}

// ---- collection and white-space processing --------------------------------

/// What collecting a run produced: its measured pieces in document order, and
/// the laid-out atomic inlines some of those pieces stand for; with where the
/// walk of the item being collected stands against that item's stretch.
struct Run {
    pieces: Vec<Piece>,
    atoms: Atoms,
    walk: Walk,
}

impl Run {
    const fn new() -> Self {
        Self {
            pieces: Vec::new(),
            atoms: Atoms::new(),
            walk: Walk::unbounded(),
        }
    }
}

/// The laid-out atomic inlines of one run, keyed by node — a node is an atom
/// at most once in a tree, and emission looks each one up by id.
struct Atoms {
    results: BTreeMap<SnapshotId, BlockResult>,
}

impl Atoms {
    const fn new() -> Self {
        Self {
            results: BTreeMap::new(),
        }
    }

    fn insert(&mut self, node: SnapshotId, result: BlockResult) {
        self.results.insert(node, result);
    }

    fn take(&mut self, node: SnapshotId) -> Option<BlockResult> {
        self.results.remove(&node)
    }
}

/// Walks one inline-level node, appending its measured pieces in document
/// order. An inline box contributes nothing itself; its descendants do. An
/// atomic inline contributes one piece for its whole box, and a `display:
/// none` node contributes nothing at all. A block-level descendant is never
/// walked into: it only moves the walk across the item's stretch, and only
/// content inside that stretch is kept.
fn collect<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    input: BlockInput,
    run: &mut Run,
) -> Result<(), CssError> {
    let styled = context.node(node_id)?;
    if styled.is_foreign_descendant() || run.walk.is_done() {
        return Ok(());
    }
    let style = styled.style();
    let display = style.display();
    if display.is_none() {
        return Ok(());
    }
    if !display.is_inline_level() {
        run.walk.cross(node_id);
        return Ok(());
    }
    if display.is_atomic_inline() {
        return collect_atom(context, styled, input, run);
    }
    let font_size = box_model::font_size_of(style, input.parent_font_size());
    if styled.text().is_none() {
        return collect_children(context, styled, input.with_parent_font_size(font_size), run);
    }
    if !run.walk.takes_content() {
        return Ok(());
    }
    collect_text(context, styled, font_size, &mut run.pieces)
}

fn collect_children<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    styled: &StyledNode,
    input: BlockInput,
    run: &mut Run,
) -> Result<(), CssError> {
    for child in styled.children().iter() {
        collect(context, child, input, run)?;
    }
    Ok(())
}

/// Lays an atomic inline out as a block container and keeps it as one piece
/// its margin box wide and tall, with its baseline on its bottom margin edge.
fn collect_atom<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    styled: &StyledNode,
    input: BlockInput,
    run: &mut Run,
) -> Result<(), CssError> {
    if !run.walk.takes_content() {
        return Ok(());
    }
    let node_id = styled.node();
    let result = block::layout_inline_block(context, node_id, input)?;
    let metrics = TextMetrics::new(result.outer_width(), result.outer_height());
    let style = styled.style();
    run.pieces.push(Piece {
        node: node_id,
        kind: PieceKind::Atomic,
        metrics,
        white_space: style.white_space(),
        text: None,
    });
    run.atoms.insert(node_id, result);
    Ok(())
}

/// The character data of `styled`, set at `font_size`.
fn collect_text<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    styled: &StyledNode,
    font_size: Au,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    let Some(run) = styled.text() else {
        return Ok(());
    };
    let style = styled.style();
    append_text(
        context,
        styled.node(),
        Setting::new(font_size, style.white_space()),
        run.as_str(),
        pieces,
    )
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

fn append_text<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
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
fn append_collapsed<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
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
fn append_border_space<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    setting: Setting,
    any: bool,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    append_space_when(context, node_id, setting, any, pieces)
}

fn append_words<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
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

fn append_separator<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    setting: Setting,
    index: usize,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    append_space_when(context, node_id, setting, index > 0, pieces)
}

/// A hyphenated word breaks after each hyphen, with the hyphen staying on the
/// left-hand part — the one UAX #14 rule this cut implements beyond the space.
fn append_word_parts<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    setting: Setting,
    word: &str,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    for part in hyphen_parts(word) {
        let metrics = context.measure(&part, setting.font_size)?;
        pieces.push(piece(node_id, PieceKind::Word, metrics, setting).drawing(&part));
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

fn append_space_when<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    setting: Setting,
    wanted: bool,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    if !wanted {
        return Ok(());
    }
    let metrics = context.measure(COLLAPSED_SPACE, setting.font_size)?;
    pieces.push(piece(node_id, PieceKind::Space, metrics, setting).drawing(COLLAPSED_SPACE));
    Ok(())
}

/// `white-space: pre`: nothing collapses, and only a `\n` breaks a line.
fn append_preserved<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
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

fn append_forced_break<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
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

fn append_preserved_line<M: TextMeasurer>(
    context: &LayoutContext<'_, M>,
    node_id: SnapshotId,
    setting: Setting,
    line: &str,
    pieces: &mut Vec<Piece>,
) -> Result<(), CssError> {
    if line.is_empty() {
        return Ok(());
    }
    let metrics = context.measure(line, setting.font_size)?;
    pieces.push(piece(node_id, PieceKind::Word, metrics, setting).drawing(line));
    Ok(())
}

const fn piece(node: SnapshotId, kind: PieceKind, metrics: TextMetrics, setting: Setting) -> Piece {
    Piece {
        node,
        kind,
        metrics,
        white_space: setting.white_space,
        text: None,
    }
}

impl Piece {
    /// This piece drawing `text`.
    fn drawing(self, text: &str) -> Self {
        Self {
            text: Some(Rc::from(text)),
            ..self
        }
    }
}

// ---- line filling ---------------------------------------------------------

/// Greedy line breaking: a word that does not fit opens a new line, and the
/// space that would have preceded it is dropped. A word wider than the whole
/// line overflows rather than being split — there is no hyphenation here.
fn fill_lines(pieces: &[Piece], limit: Au) -> Vec<Line> {
    let mut filler = LineFiller::new(limit);
    for piece in pieces {
        filler.absorb(piece.clone());
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
            PieceKind::Word | PieceKind::Atomic => self.place_word(piece),
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

    fn hold_space(&mut self, piece: Piece) {
        if self.current.is_empty() {
            return;
        }
        self.pending_space = Some(piece);
    }

    fn place_word(&mut self, piece: Piece) {
        if self.overflows(&piece) {
            self.pending_space = None;
            self.close();
        }
        self.flush_pending_space();
        self.current.push(piece);
    }

    fn overflows(&self, piece: &Piece) -> bool {
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
            .as_ref()
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
    pieces: Vec<Placed>,
}

/// One piece on its line: whose it is, where it sits, which line box holds it
/// (0-based), and the text it draws.
struct Placed {
    node: SnapshotId,
    rect: Rect,
    line: usize,
    text: Option<Rc<str>>,
}

fn place(lines: &[Line], content_width: Au, align: TextAlign) -> Placements {
    let mut cursor = Au::ZERO;
    let mut pieces = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let last = index.saturating_add(1) == lines.len();
        place_line(
            line,
            LinePlacement::new(content_width, align, cursor, last),
            index,
            &mut pieces,
        );
        cursor = cursor.saturating_add(line.height());
    }
    Placements {
        height: cursor,
        pieces,
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

fn place_line(line: &Line, placement: LinePlacement, line_index: usize, placed: &mut Vec<Placed>) {
    let mut cursor = start_offset(line, placement);
    let extra = justification(line, placement);
    for (index, piece) in line.pieces.iter().enumerate() {
        let widened = widened_width(piece, extra, index);
        let rect = piece_rect(line, piece, Point::new(cursor, placement.top));
        placed.push(Placed {
            node: piece.node,
            rect,
            line: line_index,
            text: piece.text.clone(),
        });
        cursor = cursor.saturating_add(widened);
    }
}

fn piece_rect(line: &Line, piece: &Piece, origin: Point) -> Rect {
    let metrics = piece.metrics;
    let top = origin
        .vertical()
        .saturating_add(line.ascent)
        .saturating_sub(metrics.baseline());
    rect_at(
        Point::new(origin.horizontal(), top),
        metrics.width(),
        metrics.height(),
    )
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
fn widened_width(piece: &Piece, extra: Extra, index: usize) -> Au {
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

mod emit;
mod window;

use emit::Emitter;
use window::Walk;
pub use window::{InlineItem, block_descendants};
