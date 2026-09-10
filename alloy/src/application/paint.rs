//! [`paint_box_tree`] — walks a [`LayoutBoxTree`] in document order, emitting
//! declarative [`DisplayList`] commands into a [`DisplayListBuilder`].
//!
//! ADR-0010: paint lives in `alloy::application::paint`, keeping `core/css` and
//! `core/graphics` decoupled.

use css::{EdgeSizes, LayoutBox, LayoutBoxTree, StyledNode, StyledTree};
use graphics::{
    Au, Color, DisplayListBuilder, FaceMetrics, FontId, FontProvider, GlyphId, GlyphInstance,
    GlyphRun, GraphicsError, ImageId, Point, PxRect, Rect, Size,
};

/// Default font identifier used for text painting in the headless pipeline.
pub const DEFAULT_FONT: FontId = FontId::new(1);

/// Paints all boxes in document order to the display list builder.
pub fn paint_box_tree(
    box_tree: &LayoutBoxTree,
    styled_tree: &StyledTree,
    font_provider: &dyn FontProvider,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    for laid_out in box_tree.boxes_in_document_order() {
        paint_single_box(laid_out, styled_tree, font_provider, builder)?;
    }
    Ok(())
}

fn paint_single_box(
    laid_out: &LayoutBox,
    styled_tree: &StyledTree,
    font_provider: &dyn FontProvider,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let Some(styled_node) = styled_tree.node(laid_out.node()) else {
        return Ok(());
    };

    paint_background(laid_out, styled_node, builder)?;
    paint_borders(laid_out, styled_node, builder)?;
    paint_text(laid_out, styled_node, font_provider, builder)?;
    paint_image(laid_out, builder)
}

fn paint_background(
    laid_out: &LayoutBox,
    styled_node: &StyledNode,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let background_color = styled_node.style().background_color().to_graphics();
    if background_color.is_transparent() {
        return Ok(());
    }

    let border_box = laid_out.border_box();
    if border_box.is_empty() {
        return Ok(());
    }

    builder.draw_rect(rect_to_px_rect(border_box), background_color)
}

fn paint_borders(
    laid_out: &LayoutBox,
    styled_node: &StyledNode,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let border = laid_out.border();
    let border_color = styled_node.style().color().to_graphics();
    if border_color.is_transparent() {
        return Ok(());
    }

    let border_box = laid_out.border_box();
    if border_box.is_empty() {
        return Ok(());
    }

    paint_border_sides(border_box, border, border_color, builder)
}

fn paint_border_sides(
    border_box: Rect,
    border: EdgeSizes,
    color: Color,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let min_x = border_box.min_x();
    let min_y = border_box.min_y();
    let box_width = border_box.size().width();
    let box_height = border_box.size().height();
    let max_x = border_box.max_x();
    let max_y = border_box.max_y();

    let top_w = border.top();
    let right_w = border.right();
    let bottom_w = border.bottom();
    let left_w = border.left();

    // Top border
    if !top_w.is_zero()
        && !box_width.is_zero()
        && let Some(size) = Size::new(box_width, top_w)
    {
        let rect = Rect::new(Point::new(min_x, min_y), size);
        builder.draw_rect(rect_to_px_rect(rect), color)?;
    }

    // Bottom border
    if !bottom_w.is_zero()
        && !box_width.is_zero()
        && let Some(size) = Size::new(box_width, bottom_w)
    {
        let origin = Point::new(min_x, max_y.saturating_sub(bottom_w));
        let rect = Rect::new(origin, size);
        builder.draw_rect(rect_to_px_rect(rect), color)?;
    }

    // Left border (between top and bottom to avoid corner overdraw)
    let vert_h = box_height.saturating_sub(top_w).saturating_sub(bottom_w);
    if !left_w.is_zero()
        && !vert_h.is_zero()
        && let Some(size) = Size::new(left_w, vert_h)
    {
        let origin = Point::new(min_x, min_y.saturating_add(top_w));
        let rect = Rect::new(origin, size);
        builder.draw_rect(rect_to_px_rect(rect), color)?;
    }

    // Right border (between top and bottom to avoid corner overdraw)
    if !right_w.is_zero()
        && !vert_h.is_zero()
        && let Some(size) = Size::new(right_w, vert_h)
    {
        let origin = Point::new(max_x.saturating_sub(right_w), min_y.saturating_add(top_w));
        let rect = Rect::new(origin, size);
        builder.draw_rect(rect_to_px_rect(rect), color)?;
    }

    Ok(())
}

fn paint_image(
    laid_out: &LayoutBox,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    if !laid_out.intrinsic_size().is_pending() {
        return Ok(());
    }
    let content = laid_out.content();
    if content.is_empty() {
        return Ok(());
    }
    let area = rect_to_px_rect(content);
    let raw_id = u32::try_from(laid_out.node().index()).unwrap_or(0);
    let image_id = ImageId::new(raw_id);
    builder.draw_image(image_id, area, area)
}

fn paint_text(
    laid_out: &LayoutBox,
    styled_node: &StyledNode,
    font_provider: &dyn FontProvider,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let Some(text_run) = styled_node.text() else {
        return Ok(());
    };
    if text_run.is_empty() {
        return Ok(());
    }

    let color = styled_node.style().color().to_graphics();
    if color.is_transparent() {
        return Ok(());
    }

    let content = laid_out.content();
    let metrics = font_provider.metrics(DEFAULT_FONT)?;
    let glyphs = layout_glyphs(
        text_run.as_str(),
        content,
        styled_node,
        metrics,
        font_provider,
    )?;

    if !glyphs.is_empty() {
        builder.draw_text(glyphs, color, DEFAULT_FONT)?;
    }

    Ok(())
}

fn layout_glyphs(
    text: &str,
    content: Rect,
    styled_node: &StyledNode,
    metrics: FaceMetrics,
    font_provider: &dyn FontProvider,
) -> Result<GlyphRun, GraphicsError> {
    let white_space = styled_node.style().white_space();
    let min_x = content.min_x();
    let max_x = content.max_x();
    let line_height = metrics.line_height();
    let mut baseline_y = content.min_y().saturating_add(metrics.ascent());
    let mut pen_x = min_x;
    let mut run = GlyphRun::new();

    if white_space.preserves_newlines() {
        layout_preformatted(
            text,
            min_x,
            line_height,
            metrics.ascent(),
            content.min_y(),
            font_provider,
            &mut run,
        )?;
        return Ok(run);
    }

    for word in text.split_whitespace() {
        let word_width = measure_text_run(word, font_provider)?;
        let space_glyph = font_provider.glyph_for_char(DEFAULT_FONT, ' ')?;
        let space_advance = font_provider.advance(DEFAULT_FONT, space_glyph)?;

        if pen_x > min_x {
            let next_pos = pen_x
                .saturating_add(space_advance)
                .saturating_add(word_width);
            if white_space.allows_soft_wrap() && next_pos > max_x {
                pen_x = min_x;
                baseline_y = baseline_y.saturating_add(line_height);
            } else {
                pen_x = pen_x.saturating_add(space_advance);
            }
        }

        pen_x = append_word_glyphs(word, pen_x, baseline_y, font_provider, &mut run)?;
    }

    Ok(run)
}

fn append_word_glyphs(
    word: &str,
    mut pen_x: Au,
    baseline_y: Au,
    font_provider: &dyn FontProvider,
    run: &mut GlyphRun,
) -> Result<Au, GraphicsError> {
    for ch in word.chars() {
        let glyph = font_provider.glyph_for_char(DEFAULT_FONT, ch)?;
        if glyph != GlyphId::NOTDEF {
            run.push(GlyphInstance::new(glyph, Point::new(pen_x, baseline_y)));
        }
        let advance = font_provider.advance(DEFAULT_FONT, glyph)?;
        pen_x = pen_x.saturating_add(advance);
    }
    Ok(pen_x)
}

fn measure_text_run(text: &str, font_provider: &dyn FontProvider) -> Result<Au, GraphicsError> {
    let mut total = Au::ZERO;
    for ch in text.chars() {
        let glyph = font_provider.glyph_for_char(DEFAULT_FONT, ch)?;
        let advance = font_provider.advance(DEFAULT_FONT, glyph)?;
        total = total.saturating_add(advance);
    }
    Ok(total)
}

fn layout_preformatted(
    text: &str,
    min_x: Au,
    line_height: Au,
    ascent: Au,
    min_y: Au,
    font_provider: &dyn FontProvider,
    run: &mut GlyphRun,
) -> Result<(), GraphicsError> {
    let mut baseline_y = min_y.saturating_add(ascent);
    for line in text.split('\n') {
        let mut pen_x = min_x;
        for ch in line.chars() {
            let glyph = font_provider.glyph_for_char(DEFAULT_FONT, ch)?;
            if glyph != GlyphId::NOTDEF {
                run.push(GlyphInstance::new(glyph, Point::new(pen_x, baseline_y)));
            }
            let advance = font_provider.advance(DEFAULT_FONT, glyph)?;
            pen_x = pen_x.saturating_add(advance);
        }
        baseline_y = baseline_y.saturating_add(line_height);
    }
    Ok(())
}

fn rect_to_px_rect(rect: Rect) -> PxRect {
    PxRect::new(
        rect.min_x().to_px(),
        rect.min_y().to_px(),
        rect.size().width().to_px(),
        rect.size().height().to_px(),
    )
}
