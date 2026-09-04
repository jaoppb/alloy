//! [`TtfParserProvider`] — a [`FontProvider`] backed by real font files, parsed
//! by `ttf-parser` (`unsafe`-free, `ADR-0018` row 1 — see `Cargo.toml`).
//!
//! Registration is a consuming builder (`with_face`), so the provider is
//! immutable once built: no lock is needed to satisfy `FontProvider: Send +
//! Sync`, and a bad registration is refused at build time rather than
//! discovered mid-render.
//!
//! ## Determinism, and where floating point is allowed
//!
//! `ADR-0016` keeps *layout* geometry in exact integer [`Au`]. A glyph outline
//! is Bézier curves in font-design units, and there is no exact-integer way to
//! flatten a curve or fill a polygon — every real rasterizer uses floating
//! point here. What `ADR-0016` actually requires is *reproducibility*, and
//! plain IEEE 754 `f32` arithmetic (`+ - * /`, no transcendental functions, no
//! fused-multiply-add) already gives that on every target this workspace
//! builds for. So this module uses `f32` for curve flattening and the
//! point-in-polygon fill, and nowhere else in `core/graphics`.

use core::fmt;
use std::collections::BTreeMap;

use ttf_parser::{Face, GlyphId as TtfGlyphId, OutlineBuilder};

use crate::application::FontProvider;
use crate::domain::error::GraphicsError;
use crate::domain::font::{FaceMetrics, FontId, GlyphBitmap, GlyphId};
use crate::domain::geometry::Point;
use crate::domain::unit::Au;

/// How many straight segments a curve is flattened into. Fixed, so the answer
/// is a pure function of the outline — the same reasoning `raster.rs`'s
/// `CORNER_SAMPLES` uses for arcs.
const CURVE_STEPS: u8 = 8;

/// Samples per axis when testing whether a pixel lies inside the outline —
/// mirrors `raster::CORNER_SAMPLES`.
const GLYPH_SAMPLES: u8 = 4;

/// `u32` to `f32` for glyph-space pixel coordinates.
///
/// Glyph bitmaps are at most a few hundred pixels per side (`FaceMetrics` and
/// `GraphicsError::LimitExceeded`-free by construction — no font this crate
/// registers has an absurd point size), so the conversion never loses
/// precision; that is what makes this the one place these numeric-narrowing
/// lints are waived, instead of the general case they guard against.
#[allow(clippy::as_conversions, clippy::cast_precision_loss)]
const fn pixels_to_f32(value: u32) -> f32 {
    value as f32
}

/// The inverse of [`pixels_to_f32`], for turning a computed bitmap extent back
/// into a pixel count. `value` is a non-negative, finite glyph bounding-box
/// dimension in pixels (checked by the caller), bounded the same way — see
/// [`pixels_to_f32`].
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
const fn f32_to_pixels(value: f32) -> u32 {
    value as u32
}

/// One registered face: its bytes (owned, so `Face::parse` is cheap to redo per
/// call rather than fighting a self-referential struct) and the size it was
/// registered at.
struct RegisteredFace {
    data: Vec<u8>,
    size: Au,
}

/// A [`FontProvider`] over real, `ttf-parser`-parsed font files.
#[derive(Default)]
pub struct TtfParserProvider {
    faces: BTreeMap<FontId, RegisteredFace>,
}

impl fmt::Debug for TtfParserProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TtfParserProvider")
            .field("registered", &self.faces.len())
            .finish()
    }
}

impl TtfParserProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `data` (a whole font-file's bytes) under `id`, to be
    /// rasterized at `size`. Refuses `data` `ttf-parser` cannot parse, rather
    /// than deferring the failure to the first `rasterize` call.
    pub fn with_face(mut self, id: FontId, data: Vec<u8>, size: Au) -> Result<Self, GraphicsError> {
        Face::parse(&data, 0)
            .map_err(|_parse_error| GraphicsError::FontUnavailable { font: id })?;
        self.faces.insert(id, RegisteredFace { data, size });
        Ok(self)
    }

    fn face_for(&self, id: FontId) -> Result<(Face<'_>, Au), GraphicsError> {
        let registered = self
            .faces
            .get(&id)
            .ok_or(GraphicsError::FontUnavailable { font: id })?;
        let face = Face::parse(&registered.data, 0)
            .map_err(|_parse_error| GraphicsError::FontUnavailable { font: id })?;
        Ok((face, registered.size))
    }
}

impl FontProvider for TtfParserProvider {
    fn rasterize(&self, font: FontId, glyph: GlyphId) -> Result<GlyphBitmap, GraphicsError> {
        let (face, size) = self.face_for(font)?;
        Ok(rasterize_glyph(&face, glyph, size))
    }

    fn metrics(&self, font: FontId) -> Result<FaceMetrics, GraphicsError> {
        let (face, size) = self.face_for(font)?;
        Ok(face_metrics(&face, size))
    }

    fn glyph_for_char(&self, font: FontId, character: char) -> Result<GlyphId, GraphicsError> {
        let (face, _size) = self.face_for(font)?;
        let mapped = face
            .glyph_index(character)
            .map_or(GlyphId::NOTDEF, |ttf_id| GlyphId::new(ttf_id.0));
        Ok(mapped)
    }

    fn advance(&self, font: FontId, glyph: GlyphId) -> Result<Au, GraphicsError> {
        let (face, size) = self.face_for(font)?;
        let Some(scale) = units_per_em_scale(&face, size) else {
            return Ok(Au::ZERO);
        };
        let ttf_id = TtfGlyphId(glyph.get());
        let font_units = face.glyph_hor_advance(ttf_id).unwrap_or(0);
        Ok(scaled_au(f32::from(font_units), scale))
    }
}

fn units_per_em_scale(face: &Face<'_>, size: Au) -> Option<f32> {
    let units_per_em = f32::from(face.units_per_em());
    if units_per_em <= 0.0 {
        return None;
    }
    Some(size.to_px().get() / units_per_em)
}

fn face_metrics(face: &Face<'_>, size: Au) -> FaceMetrics {
    let Some(scale) = units_per_em_scale(face, size) else {
        return FaceMetrics::default();
    };
    let ascent = scaled_au(f32::from(face.ascender()), scale);
    let descent = scaled_au(f32::from(face.descender()).abs(), scale);
    let line_gap = scaled_au(f32::from(face.line_gap()), scale);
    FaceMetrics::new(ascent, descent, line_gap)
}

fn scaled_au(font_units: f32, scale: f32) -> Au {
    let pixels = font_units * scale;
    if !pixels.is_finite() {
        return Au::ZERO;
    }
    Au::from_px(crate::domain::unit::Px::new(pixels)).unwrap_or(Au::ZERO)
}

fn rasterize_glyph(face: &Face<'_>, glyph: GlyphId, size: Au) -> GlyphBitmap {
    let Some(scale) = units_per_em_scale(face, size) else {
        return GlyphBitmap::empty();
    };
    let mut collector = OutlineCollector::new();
    let ttf_id = TtfGlyphId(glyph.get());
    let Some(bbox) = face.outline_glyph(ttf_id, &mut collector) else {
        return GlyphBitmap::empty();
    };
    collector.close_current();
    if collector.contours.is_empty() {
        return GlyphBitmap::empty();
    }
    fill_outline(&collector.contours, bbox, scale)
}

/// A closed polygon in font-design units, flattened from the outline's
/// (possibly curved) segments.
type Contour = Vec<(f32, f32)>;

/// Collects an outline's contours, flattening quadratic and cubic curves into
/// straight segments as they arrive.
struct OutlineCollector {
    contours: Vec<Contour>,
    current: Contour,
    cursor: (f32, f32),
}

impl OutlineCollector {
    const fn new() -> Self {
        Self {
            contours: Vec::new(),
            current: Vec::new(),
            cursor: (0.0, 0.0),
        }
    }

    fn close_current(&mut self) {
        if self.current.len() >= 2 {
            self.contours.push(std::mem::take(&mut self.current));
        }
        self.current.clear();
    }
}

impl OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close_current();
        self.current.push((x, y));
        self.cursor = (x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push((x, y));
        self.cursor = (x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let start = self.cursor;
        for step in 1..=CURVE_STEPS {
            let t = f32::from(step) / f32::from(CURVE_STEPS);
            self.current.push(quad_point(start, (x1, y1), (x, y), t));
        }
        self.cursor = (x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let start = self.cursor;
        for step in 1..=CURVE_STEPS {
            let t = f32::from(step) / f32::from(CURVE_STEPS);
            self.current
                .push(cubic_point(start, (x1, y1), (x2, y2), (x, y), t));
        }
        self.cursor = (x, y);
    }

    fn close(&mut self) {
        self.close_current();
    }
}

/// Evaluates a quadratic Bézier at `t`. `mul_add` would shave a rounding step
/// per term, but turns a textbook formula into an unreadable one for no
/// coverage-relevant precision gain — a rasterizer's AA coverage does not need
/// the last bit of `f32` accuracy.
#[allow(clippy::suboptimal_flops)]
fn quad_point(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let x = u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0;
    let y = u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1;
    (x, y)
}

/// Evaluates a cubic Bézier at `t`. See [`quad_point`] for why `mul_add` is
/// waived here too.
#[allow(clippy::suboptimal_flops)]
fn cubic_point(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    t: f32,
) -> (f32, f32) {
    let u = 1.0 - t;
    let x = u * u * u * p0.0 + 3.0 * u * u * t * p1.0 + 3.0 * u * t * t * p2.0 + t * t * t * p3.0;
    let y = u * u * u * p0.1 + 3.0 * u * u * t * p1.1 + 3.0 * u * t * t * p2.1 + t * t * t * p3.1;
    (x, y)
}

/// Rasterizes `contours` (font-design units) into a coverage mask sized to
/// `bbox` scaled by `scale`, using `GLYPH_SAMPLES²` supersampling and the
/// standard signed-crossing winding-number test (Sunday's algorithm) — the
/// non-zero fill rule every outline font format assumes.
fn fill_outline(contours: &[Contour], bbox: ttf_parser::Rect, scale: f32) -> GlyphBitmap {
    let origin_x = f32::from(bbox.x_min) * scale;
    let origin_y = f32::from(bbox.y_max) * scale;
    let width_px = ((f32::from(bbox.x_max) - f32::from(bbox.x_min)) * scale).ceil();
    let height_px = ((f32::from(bbox.y_max) - f32::from(bbox.y_min)) * scale).ceil();
    let width = finite_extent(width_px);
    let height = finite_extent(height_px);
    if width == 0 || height == 0 {
        return GlyphBitmap::empty();
    }
    let scaled: Vec<Contour> = contours
        .iter()
        .map(|contour| {
            contour
                .iter()
                .map(|&(x, y)| (x * scale, y * scale))
                .collect()
        })
        .collect();
    let mut coverage = vec![0_u8; area(width, height)];
    for row in 0..height {
        for column in 0..width {
            let sample_origin = (
                origin_x + pixels_to_f32(column),
                origin_y - pixels_to_f32(row),
            );
            let value = sample_pixel(&scaled, sample_origin);
            let slot = pixel_index(width, column, row).and_then(|index| coverage.get_mut(index));
            if let Some(slot) = slot {
                *slot = value;
            }
        }
    }
    let bearing = Point::new(
        Au::from_px(crate::domain::unit::Px::new(origin_x)).unwrap_or(Au::ZERO),
        Au::from_px(crate::domain::unit::Px::new(-origin_y)).unwrap_or(Au::ZERO),
    );
    GlyphBitmap::new(width, height, coverage, bearing).unwrap_or_else(GlyphBitmap::empty)
}

fn finite_extent(value: f32) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    f32_to_pixels(value)
}

fn area(width: u32, height: u32) -> usize {
    usize::try_from(width)
        .ok()
        .zip(usize::try_from(height).ok())
        .and_then(|(width, height)| width.checked_mul(height))
        .unwrap_or(0)
}

fn pixel_index(width: u32, column: u32, row: u32) -> Option<usize> {
    let width = usize::try_from(width).ok()?;
    let column = usize::try_from(column).ok()?;
    let row = usize::try_from(row).ok()?;
    row.checked_mul(width)?.checked_add(column)
}

/// The coverage of one pixel whose top-left sample origin is `origin`, in
/// `0..=255`.
fn sample_pixel(contours: &[Contour], origin: (f32, f32)) -> u8 {
    let mut inside = 0_u32;
    for sample_row in 0..GLYPH_SAMPLES {
        for sample_column in 0..GLYPH_SAMPLES {
            let point = sub_sample(origin, sample_column, sample_row);
            if winding_number(contours, point) != 0 {
                inside = inside.saturating_add(1);
            }
        }
    }
    let total = u32::from(GLYPH_SAMPLES).saturating_mul(u32::from(GLYPH_SAMPLES));
    scale_to_byte(inside, total)
}

fn sub_sample(origin: (f32, f32), column: u8, row: u8) -> (f32, f32) {
    let cell = 1.0 / f32::from(GLYPH_SAMPLES);
    let offset_x = (f32::from(column) + 0.5) * cell;
    let offset_y = (f32::from(row) + 0.5) * cell;
    (origin.0 + offset_x, origin.1 - offset_y)
}

/// Dan Sunday's winding-number test: nonzero means inside, under the nonzero
/// fill rule.
fn winding_number(contours: &[Contour], point: (f32, f32)) -> i32 {
    let mut winding = 0_i32;
    for contour in contours {
        accumulate_winding(contour, point, &mut winding);
    }
    winding
}

fn accumulate_winding(contour: &Contour, point: (f32, f32), winding: &mut i32) {
    let Some(&first) = contour.first() else {
        return;
    };
    let starts = contour.iter().copied();
    let ends = contour
        .iter()
        .copied()
        .skip(1)
        .chain(core::iter::once(first));
    for (start, end) in starts.zip(ends) {
        accumulate_edge(start, end, point, winding);
    }
}

fn accumulate_edge(start: (f32, f32), end: (f32, f32), point: (f32, f32), winding: &mut i32) {
    if start.1 <= point.1 {
        if end.1 > point.1 && is_left(start, end, point) > 0.0 {
            *winding = winding.saturating_add(1);
        }
    } else if end.1 <= point.1 && is_left(start, end, point) < 0.0 {
        *winding = winding.saturating_sub(1);
    }
}

/// Signed area of the triangle `(a, b, point)` — positive when `point` is left
/// of the directed edge `a -> b`. See [`quad_point`] for why `mul_add` is
/// waived.
#[allow(clippy::suboptimal_flops)]
fn is_left(a: (f32, f32), b: (f32, f32), point: (f32, f32)) -> f32 {
    (b.0 - a.0) * (point.1 - a.1) - (point.0 - a.0) * (b.1 - a.1)
}

fn scale_to_byte(value: u32, total: u32) -> u8 {
    if total == 0 {
        return 0;
    }
    let scaled = value
        .saturating_mul(u32::from(u8::MAX))
        .checked_div(total)
        .unwrap_or(0);
    u8::try_from(scaled).unwrap_or(u8::MAX)
}
