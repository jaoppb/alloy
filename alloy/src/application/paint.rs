//! [`paint_box_tree`] — walks a [`LayoutBoxTree`] in document order, emitting
//! declarative [`DisplayList`] commands into a [`DisplayListBuilder`].
//!
//! ADR-0010: paint lives in `alloy::application::paint`, keeping `core/css` and
//! `core/graphics` decoupled.

use browser::ImageStore;
use css::domain::color::CssColor;
use css::domain::computed::visual::{BorderColorEdges, BorderStyle, BorderStyleEdges};
use css::{EdgeSizes, INITIAL_FONT_SIZE, LayoutBox, LayoutBoxTree, StyledNode, StyledTree};
use graphics::{
    Au, Color, DisplayListBuilder, FontId, FontProvider, GlyphId, GlyphInstance, GlyphRun,
    GraphicsError, ImageId, Point, PxRect, Rect, Size,
};

/// Default font identifier used for text painting in the headless pipeline.
pub const DEFAULT_FONT: FontId = FontId::new(1);

/// Paints all boxes in document order to the display list builder.
pub fn paint_box_tree<F: FontProvider>(
    box_tree: &LayoutBoxTree,
    styled_tree: &StyledTree,
    images: &ImageStore,
    font_provider: &F,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    for laid_out in box_tree.boxes_in_document_order() {
        paint_single_box(laid_out, styled_tree, images, font_provider, builder)?;
    }
    Ok(())
}

fn paint_single_box<F: FontProvider>(
    laid_out: &LayoutBox,
    styled_tree: &StyledTree,
    images: &ImageStore,
    font_provider: &F,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let Some(styled_node) = styled_tree.node(laid_out.node()) else {
        return Ok(());
    };

    if !styled_node.style().visibility().is_visible() {
        return Ok(());
    }

    paint_background(laid_out, styled_node, builder)?;
    paint_borders(laid_out, styled_node, builder)?;
    paint_outline(laid_out, styled_node, builder)?;
    paint_text(laid_out, styled_node, font_provider, builder)?;
    paint_image(laid_out, images, builder)
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
    let border_box = laid_out.border_box();
    if border_box.is_empty() {
        return Ok(());
    }

    let border = laid_out.border();
    let text_color = styled_node.style().color();
    let visual = styled_node.style().visual();
    let styles = visual.border_styles();
    let colors = visual.border_colors();

    paint_border_sides(border_box, border, styles, colors, text_color, builder)
}

fn paint_outline(
    laid_out: &LayoutBox,
    styled_node: &StyledNode,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let outline = styled_node.style().outline();
    if !outline.style().is_drawn() {
        return Ok(());
    }

    let outline_color = outline.color().to_graphics();
    if outline_color.is_transparent() {
        return Ok(());
    }

    let border_box = laid_out.border_box();
    if border_box.is_empty() {
        return Ok(());
    }

    let font_size = styled_node.style().computed_font_size(INITIAL_FONT_SIZE);
    let container_width = laid_out.content().size().width();

    let Some(stroke_width) = outline.width().resolve_to_au(font_size, container_width) else {
        return Ok(());
    };
    if stroke_width.is_zero() || stroke_width.is_negative() {
        return Ok(());
    }

    let Some(outline_offset) = outline.offset().resolve_to_au(font_size, container_width) else {
        return Ok(());
    };

    let offset_plus_width = outline_offset.saturating_add(stroke_width);
    let double_expansion = offset_plus_width.saturating_add(offset_plus_width);

    let outer_width = border_box.size().width().saturating_add(double_expansion);
    let outer_height = border_box.size().height().saturating_add(double_expansion);
    let Some(outer_size) = Size::new(outer_width, outer_height) else {
        return Ok(());
    };

    let outer_origin = Point::new(
        border_box.min_x().saturating_sub(offset_plus_width),
        border_box.min_y().saturating_sub(offset_plus_width),
    );
    let outer_box = Rect::new(outer_origin, outer_size);

    paint_outline_sides(outer_box, stroke_width, outline_color, builder)
}

fn paint_outline_sides(
    outer_box: Rect,
    stroke_width: Au,
    outline_color: Color,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let min_x = outer_box.min_x();
    let min_y = outer_box.min_y();
    let box_width = outer_box.size().width();
    let box_height = outer_box.size().height();
    let max_x = outer_box.max_x();
    let max_y = outer_box.max_y();

    let top_origin = Point::new(min_x, min_y);
    paint_horizontal_border(
        builder,
        top_origin,
        box_width,
        stroke_width,
        BorderStyle::Solid,
        outline_color,
    )?;

    let bottom_origin = Point::new(min_x, max_y.saturating_sub(stroke_width));
    paint_horizontal_border(
        builder,
        bottom_origin,
        box_width,
        stroke_width,
        BorderStyle::Solid,
        outline_color,
    )?;

    let vert_h = box_height
        .saturating_sub(stroke_width)
        .saturating_sub(stroke_width);
    let left_origin = Point::new(min_x, min_y.saturating_add(stroke_width));
    paint_vertical_border(
        builder,
        left_origin,
        stroke_width,
        vert_h,
        BorderStyle::Solid,
        outline_color,
    )?;

    let right_origin = Point::new(
        max_x.saturating_sub(stroke_width),
        min_y.saturating_add(stroke_width),
    );
    paint_vertical_border(
        builder,
        right_origin,
        stroke_width,
        vert_h,
        BorderStyle::Solid,
        outline_color,
    )
}

fn paint_border_sides(
    border_box: Rect,
    border: EdgeSizes,
    styles: BorderStyleEdges,
    colors: BorderColorEdges,
    text_color: CssColor,
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
    let top_origin = Point::new(min_x, min_y);
    let top_color = colors.top().resolve(text_color).to_graphics();
    paint_horizontal_border(
        builder,
        top_origin,
        box_width,
        top_w,
        styles.top(),
        top_color,
    )?;

    // Bottom border
    let bottom_origin = Point::new(min_x, max_y.saturating_sub(bottom_w));
    let bottom_color = colors.bottom().resolve(text_color).to_graphics();
    paint_horizontal_border(
        builder,
        bottom_origin,
        box_width,
        bottom_w,
        styles.bottom(),
        bottom_color,
    )?;

    // Left border (between top and bottom to avoid corner overdraw)
    let vert_h = box_height.saturating_sub(top_w).saturating_sub(bottom_w);
    let left_origin = Point::new(min_x, min_y.saturating_add(top_w));
    let left_color = colors.left().resolve(text_color).to_graphics();
    paint_vertical_border(
        builder,
        left_origin,
        left_w,
        vert_h,
        styles.left(),
        left_color,
    )?;

    // Right border (between top and bottom to avoid corner overdraw)
    let right_origin = Point::new(max_x.saturating_sub(right_w), min_y.saturating_add(top_w));
    let right_color = colors.right().resolve(text_color).to_graphics();
    paint_vertical_border(
        builder,
        right_origin,
        right_w,
        vert_h,
        styles.right(),
        right_color,
    )?;

    Ok(())
}

fn paint_horizontal_border(
    builder: &mut DisplayListBuilder,
    origin: Point,
    width: Au,
    thickness: Au,
    style: BorderStyle,
    color: Color,
) -> Result<(), GraphicsError> {
    if thickness.is_zero() || width.is_zero() || style.is_none_or_hidden() || color.is_transparent()
    {
        return Ok(());
    }
    let Some(size) = Size::new(width, thickness) else {
        return Ok(());
    };
    builder.draw_rect(rect_to_px_rect(Rect::new(origin, size)), color)
}

fn paint_vertical_border(
    builder: &mut DisplayListBuilder,
    origin: Point,
    thickness: Au,
    height: Au,
    style: BorderStyle,
    color: Color,
) -> Result<(), GraphicsError> {
    if thickness.is_zero()
        || height.is_zero()
        || style.is_none_or_hidden()
        || color.is_transparent()
    {
        return Ok(());
    }
    let Some(size) = Size::new(thickness, height) else {
        return Ok(());
    };
    builder.draw_rect(rect_to_px_rect(Rect::new(origin, size)), color)
}

fn paint_image(
    laid_out: &LayoutBox,
    images: &ImageStore,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let raw_id = u32::try_from(laid_out.node().index()).unwrap_or(u32::MAX);
    let image_id = ImageId::new(raw_id);
    if !images.contains(image_id) {
        return Ok(());
    }
    let content = laid_out.content();
    if content.is_empty() {
        return Ok(());
    }
    let area = rect_to_px_rect(content);
    builder.draw_image(image_id, area, area)
}

/// Draws the text layout placed on this fragment's line, as one line from the
/// fragment's origin (#134): layout already broke the lines, so paint never
/// wraps again — which is what keeps a run that starts mid-line, or after a
/// space, where layout put it.
fn paint_text<F: FontProvider>(
    laid_out: &LayoutBox,
    styled_node: &StyledNode,
    font_provider: &F,
    builder: &mut DisplayListBuilder,
) -> Result<(), GraphicsError> {
    let Some(text) = laid_out.text() else {
        return Ok(());
    };
    let color = styled_node.style().color().to_graphics();
    if color.is_transparent() {
        return Ok(());
    }

    let content = laid_out.content();
    let metrics = font_provider.metrics(DEFAULT_FONT)?;
    let baseline = content.min_y().saturating_add(metrics.ascent());
    let mut glyphs = GlyphRun::new();
    append_line_glyphs(
        text.as_str(),
        Point::new(content.min_x(), baseline),
        font_provider,
        &mut glyphs,
    )?;

    if glyphs.is_empty() {
        return Ok(());
    }
    builder.draw_text(glyphs, color, DEFAULT_FONT)
}

/// Appends one glyph per character of `line`, advancing the pen from `origin`
/// (whose vertical coordinate is the baseline).
fn append_line_glyphs<F: FontProvider>(
    line: &str,
    origin: Point,
    font_provider: &F,
    run: &mut GlyphRun,
) -> Result<(), GraphicsError> {
    let baseline = origin.vertical();
    let mut pen_x = origin.horizontal();
    for character in line.chars() {
        let glyph = font_provider.glyph_for_char(DEFAULT_FONT, character)?;
        if glyph != GlyphId::NOTDEF {
            run.push(GlyphInstance::new(glyph, Point::new(pen_x, baseline)));
        }
        let advance = font_provider.advance(DEFAULT_FONT, glyph)?;
        pen_x = pen_x.saturating_add(advance);
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
