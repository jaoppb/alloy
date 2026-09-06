//! Conformance and unit tests for CSS Logical Properties & Values L1.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

#[path = "../src/domain/computed/logical.rs"]
pub mod logical;

pub mod domain {
    pub use css::domain::*;
    pub mod computed {
        pub use css::domain::computed::*;
        pub mod logical {
            pub use crate::logical::*;
        }
    }
}
pub use css::infrastructure;

#[path = "../src/infrastructure/cascade/logical_values.rs"]
pub mod logical_values;

use css::ComputedStyle;
use css::Length;
use css::infrastructure::parser::token::Token;
use css::infrastructure::parser::tokenize;
use logical::{
    Direction, LogicalAxis, LogicalEdges, LogicalInsets, LogicalSide, LogicalSizing, LogicalStyle,
    PhysicalAxis, PhysicalSide, WritingContext, WritingMode,
};

fn tokens(input: &str) -> Vec<Token> {
    tokenize(input)
        .iter()
        .map(|spanned| spanned.token().clone())
        .filter(|token| !token.is_whitespace())
        .collect()
}

const fn px(val: f32) -> Length {
    Length::Pixels(val)
}

// =========================================================================
// 1. WritingMode and Direction enum tests
// =========================================================================

#[test]
fn test_writing_mode_values() {
    let horizontal = WritingMode::HorizontalTb;
    assert!(horizontal.is_horizontal());
    assert!(!horizontal.is_vertical());
    assert_eq!(horizontal.keyword(), "horizontal-tb");
    assert_eq!(format!("{horizontal}"), "horizontal-tb");

    let v_rl = WritingMode::VerticalRl;
    assert!(!v_rl.is_horizontal());
    assert!(v_rl.is_vertical());
    assert_eq!(v_rl.keyword(), "vertical-rl");
    assert_eq!(format!("{v_rl}"), "vertical-rl");

    let v_lr = WritingMode::VerticalLr;
    assert!(!v_lr.is_horizontal());
    assert!(v_lr.is_vertical());
    assert_eq!(v_lr.keyword(), "vertical-lr");
    assert_eq!(format!("{v_lr}"), "vertical-lr");
}

#[test]
fn test_direction_values() {
    let ltr = Direction::Ltr;
    assert!(ltr.is_ltr());
    assert!(!ltr.is_rtl());
    assert_eq!(ltr.keyword(), "ltr");
    assert_eq!(format!("{ltr}"), "ltr");

    let rtl = Direction::Rtl;
    assert!(!rtl.is_ltr());
    assert!(rtl.is_rtl());
    assert_eq!(rtl.keyword(), "rtl");
    assert_eq!(format!("{rtl}"), "rtl");
}

// =========================================================================
// 2. Logical to Physical Mapping Matrix (6 combinations)
// =========================================================================

#[test]
fn test_mapping_horizontal_tb_ltr() {
    let ctx = WritingContext::new(WritingMode::HorizontalTb, Direction::Ltr);
    assert_eq!(ctx.map_axis(LogicalAxis::Block), PhysicalAxis::Vertical);
    assert_eq!(ctx.map_axis(LogicalAxis::Inline), PhysicalAxis::Horizontal);

    assert_eq!(ctx.map_side(LogicalSide::BlockStart), PhysicalSide::Top);
    assert_eq!(ctx.map_side(LogicalSide::BlockEnd), PhysicalSide::Bottom);
    assert_eq!(ctx.map_side(LogicalSide::InlineStart), PhysicalSide::Left);
    assert_eq!(ctx.map_side(LogicalSide::InlineEnd), PhysicalSide::Right);
}

#[test]
fn test_mapping_horizontal_tb_rtl() {
    let ctx = WritingContext::new(WritingMode::HorizontalTb, Direction::Rtl);
    assert_eq!(ctx.map_axis(LogicalAxis::Block), PhysicalAxis::Vertical);
    assert_eq!(ctx.map_axis(LogicalAxis::Inline), PhysicalAxis::Horizontal);

    assert_eq!(ctx.map_side(LogicalSide::BlockStart), PhysicalSide::Top);
    assert_eq!(ctx.map_side(LogicalSide::BlockEnd), PhysicalSide::Bottom);
    assert_eq!(ctx.map_side(LogicalSide::InlineStart), PhysicalSide::Right);
    assert_eq!(ctx.map_side(LogicalSide::InlineEnd), PhysicalSide::Left);
}

#[test]
fn test_mapping_vertical_rl_ltr() {
    let ctx = WritingContext::new(WritingMode::VerticalRl, Direction::Ltr);
    assert_eq!(ctx.map_axis(LogicalAxis::Block), PhysicalAxis::Horizontal);
    assert_eq!(ctx.map_axis(LogicalAxis::Inline), PhysicalAxis::Vertical);

    assert_eq!(ctx.map_side(LogicalSide::BlockStart), PhysicalSide::Right);
    assert_eq!(ctx.map_side(LogicalSide::BlockEnd), PhysicalSide::Left);
    assert_eq!(ctx.map_side(LogicalSide::InlineStart), PhysicalSide::Top);
    assert_eq!(ctx.map_side(LogicalSide::InlineEnd), PhysicalSide::Bottom);
}

#[test]
fn test_mapping_vertical_rl_rtl() {
    let ctx = WritingContext::new(WritingMode::VerticalRl, Direction::Rtl);
    assert_eq!(ctx.map_axis(LogicalAxis::Block), PhysicalAxis::Horizontal);
    assert_eq!(ctx.map_axis(LogicalAxis::Inline), PhysicalAxis::Vertical);

    assert_eq!(ctx.map_side(LogicalSide::BlockStart), PhysicalSide::Right);
    assert_eq!(ctx.map_side(LogicalSide::BlockEnd), PhysicalSide::Left);
    assert_eq!(ctx.map_side(LogicalSide::InlineStart), PhysicalSide::Bottom);
    assert_eq!(ctx.map_side(LogicalSide::InlineEnd), PhysicalSide::Top);
}

#[test]
fn test_mapping_vertical_lr_ltr() {
    let ctx = WritingContext::new(WritingMode::VerticalLr, Direction::Ltr);
    assert_eq!(ctx.map_axis(LogicalAxis::Block), PhysicalAxis::Horizontal);
    assert_eq!(ctx.map_axis(LogicalAxis::Inline), PhysicalAxis::Vertical);

    assert_eq!(ctx.map_side(LogicalSide::BlockStart), PhysicalSide::Left);
    assert_eq!(ctx.map_side(LogicalSide::BlockEnd), PhysicalSide::Right);
    assert_eq!(ctx.map_side(LogicalSide::InlineStart), PhysicalSide::Top);
    assert_eq!(ctx.map_side(LogicalSide::InlineEnd), PhysicalSide::Bottom);
}

#[test]
fn test_mapping_vertical_lr_rtl() {
    let ctx = WritingContext::new(WritingMode::VerticalLr, Direction::Rtl);
    assert_eq!(ctx.map_axis(LogicalAxis::Block), PhysicalAxis::Horizontal);
    assert_eq!(ctx.map_axis(LogicalAxis::Inline), PhysicalAxis::Vertical);

    assert_eq!(ctx.map_side(LogicalSide::BlockStart), PhysicalSide::Left);
    assert_eq!(ctx.map_side(LogicalSide::BlockEnd), PhysicalSide::Right);
    assert_eq!(ctx.map_side(LogicalSide::InlineStart), PhysicalSide::Bottom);
    assert_eq!(ctx.map_side(LogicalSide::InlineEnd), PhysicalSide::Top);
}

// =========================================================================
// 3. Logical Sizing Tests
// =========================================================================

#[test]
fn test_logical_sizing_horizontal() {
    let ctx = WritingContext::new(WritingMode::HorizontalTb, Direction::Ltr);
    let mut style = ComputedStyle::initial();

    style =
        logical_values::apply_with_context(style, ctx, "inline-size", &tokens("100px")).unwrap();
    style = logical_values::apply_with_context(style, ctx, "block-size", &tokens("200px")).unwrap();

    assert_eq!(
        style.width(),
        css::domain::computed::sizing::Sizing::Fixed(px(100.0))
    );
    assert_eq!(
        style.height(),
        css::domain::computed::sizing::Sizing::Fixed(px(200.0))
    );

    style =
        logical_values::apply_with_context(style, ctx, "min-inline-size", &tokens("50px")).unwrap();
    style = logical_values::apply_with_context(style, ctx, "max-inline-size", &tokens("150px"))
        .unwrap();
    style =
        logical_values::apply_with_context(style, ctx, "min-block-size", &tokens("80px")).unwrap();
    style =
        logical_values::apply_with_context(style, ctx, "max-block-size", &tokens("250px")).unwrap();

    let c = style.constraints();
    assert_eq!(
        c.min_width(),
        css::domain::computed::sizing::Sizing::Fixed(px(50.0))
    );
    assert_eq!(
        c.max_width(),
        css::domain::computed::sizing::Sizing::Fixed(px(150.0))
    );
    assert_eq!(
        c.min_height(),
        css::domain::computed::sizing::Sizing::Fixed(px(80.0))
    );
    assert_eq!(
        c.max_height(),
        css::domain::computed::sizing::Sizing::Fixed(px(250.0))
    );
}

#[test]
fn test_logical_sizing_vertical() {
    let ctx = WritingContext::new(WritingMode::VerticalRl, Direction::Ltr);
    let mut style = ComputedStyle::initial();

    style =
        logical_values::apply_with_context(style, ctx, "inline-size", &tokens("100px")).unwrap();
    style = logical_values::apply_with_context(style, ctx, "block-size", &tokens("200px")).unwrap();

    // In vertical writing mode: inline-size -> height, block-size -> width
    assert_eq!(
        style.height(),
        css::domain::computed::sizing::Sizing::Fixed(px(100.0))
    );
    assert_eq!(
        style.width(),
        css::domain::computed::sizing::Sizing::Fixed(px(200.0))
    );

    style =
        logical_values::apply_with_context(style, ctx, "min-inline-size", &tokens("50px")).unwrap();
    style = logical_values::apply_with_context(style, ctx, "max-inline-size", &tokens("150px"))
        .unwrap();
    style =
        logical_values::apply_with_context(style, ctx, "min-block-size", &tokens("80px")).unwrap();
    style =
        logical_values::apply_with_context(style, ctx, "max-block-size", &tokens("250px")).unwrap();

    let c = style.constraints();
    assert_eq!(
        c.min_height(),
        css::domain::computed::sizing::Sizing::Fixed(px(50.0))
    );
    assert_eq!(
        c.max_height(),
        css::domain::computed::sizing::Sizing::Fixed(px(150.0))
    );
    assert_eq!(
        c.min_width(),
        css::domain::computed::sizing::Sizing::Fixed(px(80.0))
    );
    assert_eq!(
        c.max_width(),
        css::domain::computed::sizing::Sizing::Fixed(px(250.0))
    );
}

// =========================================================================
// 4. Logical Margins Tests
// =========================================================================

#[test]
fn test_logical_margins_ltr_and_rtl() {
    let ltr = WritingContext::new(WritingMode::HorizontalTb, Direction::Ltr);
    let rtl = WritingContext::new(WritingMode::HorizontalTb, Direction::Rtl);

    let mut style_ltr = ComputedStyle::initial();
    style_ltr =
        logical_values::apply_with_context(style_ltr, ltr, "margin-inline-start", &tokens("10px"))
            .unwrap();
    style_ltr =
        logical_values::apply_with_context(style_ltr, ltr, "margin-inline-end", &tokens("20px"))
            .unwrap();
    style_ltr =
        logical_values::apply_with_context(style_ltr, ltr, "margin-block-start", &tokens("30px"))
            .unwrap();
    style_ltr =
        logical_values::apply_with_context(style_ltr, ltr, "margin-block-end", &tokens("40px"))
            .unwrap();

    assert_eq!(style_ltr.margin().left(), px(10.0));
    assert_eq!(style_ltr.margin().right(), px(20.0));
    assert_eq!(style_ltr.margin().top(), px(30.0));
    assert_eq!(style_ltr.margin().bottom(), px(40.0));

    let mut style_rtl = ComputedStyle::initial();
    style_rtl =
        logical_values::apply_with_context(style_rtl, rtl, "margin-inline-start", &tokens("10px"))
            .unwrap();
    style_rtl =
        logical_values::apply_with_context(style_rtl, rtl, "margin-inline-end", &tokens("20px"))
            .unwrap();
    style_rtl =
        logical_values::apply_with_context(style_rtl, rtl, "margin-block-start", &tokens("30px"))
            .unwrap();
    style_rtl =
        logical_values::apply_with_context(style_rtl, rtl, "margin-block-end", &tokens("40px"))
            .unwrap();

    // In RTL: inline-start -> right, inline-end -> left
    assert_eq!(style_rtl.margin().right(), px(10.0));
    assert_eq!(style_rtl.margin().left(), px(20.0));
    assert_eq!(style_rtl.margin().top(), px(30.0));
    assert_eq!(style_rtl.margin().bottom(), px(40.0));
}

#[test]
fn test_logical_margin_shorthands() {
    let ctx = WritingContext::new(WritingMode::HorizontalTb, Direction::Ltr);
    let mut style = ComputedStyle::initial();

    // 1 value shorthand
    style =
        logical_values::apply_with_context(style, ctx, "margin-inline", &tokens("15px")).unwrap();
    assert_eq!(style.margin().left(), px(15.0));
    assert_eq!(style.margin().right(), px(15.0));

    // 2 value shorthand: start end
    style = logical_values::apply_with_context(style, ctx, "margin-block", &tokens("10px 20px"))
        .unwrap();
    assert_eq!(style.margin().top(), px(10.0));
    assert_eq!(style.margin().bottom(), px(20.0));
}

// =========================================================================
// 5. Logical Padding Tests
// =========================================================================

#[test]
fn test_logical_padding_vertical_rl() {
    let ctx = WritingContext::new(WritingMode::VerticalRl, Direction::Ltr);
    let mut style = ComputedStyle::initial();

    style = logical_values::apply_with_context(style, ctx, "padding-block-start", &tokens("5px"))
        .unwrap();
    style = logical_values::apply_with_context(style, ctx, "padding-block-end", &tokens("10px"))
        .unwrap();
    style = logical_values::apply_with_context(style, ctx, "padding-inline-start", &tokens("15px"))
        .unwrap();
    style = logical_values::apply_with_context(style, ctx, "padding-inline-end", &tokens("20px"))
        .unwrap();

    // In vertical-rl ltr: block-start -> right, block-end -> left, inline-start -> top, inline-end -> bottom
    assert_eq!(style.padding().right(), px(5.0));
    assert_eq!(style.padding().left(), px(10.0));
    assert_eq!(style.padding().top(), px(15.0));
    assert_eq!(style.padding().bottom(), px(20.0));
}

#[test]
fn test_logical_padding_shorthands() {
    let ctx = WritingContext::new(WritingMode::HorizontalTb, Direction::Ltr);
    let mut style = ComputedStyle::initial();

    style = logical_values::apply_with_context(style, ctx, "padding-inline", &tokens("8px 12px"))
        .unwrap();
    assert_eq!(style.padding().left(), px(8.0));
    assert_eq!(style.padding().right(), px(12.0));

    style =
        logical_values::apply_with_context(style, ctx, "padding-block", &tokens("16px")).unwrap();
    assert_eq!(style.padding().top(), px(16.0));
    assert_eq!(style.padding().bottom(), px(16.0));
}

// =========================================================================
// 6. Logical Borders Tests
// =========================================================================

#[test]
fn test_logical_border_widths() {
    let ctx = WritingContext::new(WritingMode::HorizontalTb, Direction::Ltr);
    let mut style = ComputedStyle::initial();

    style =
        logical_values::apply_with_context(style, ctx, "border-inline-start-width", &tokens("1px"))
            .unwrap();
    style =
        logical_values::apply_with_context(style, ctx, "border-inline-end-width", &tokens("2px"))
            .unwrap();
    style =
        logical_values::apply_with_context(style, ctx, "border-block-start-width", &tokens("3px"))
            .unwrap();
    style =
        logical_values::apply_with_context(style, ctx, "border-block-end-width", &tokens("4px"))
            .unwrap();

    assert_eq!(style.border().left(), px(1.0));
    assert_eq!(style.border().right(), px(2.0));
    assert_eq!(style.border().top(), px(3.0));
    assert_eq!(style.border().bottom(), px(4.0));
}

#[test]
fn test_logical_border_shorthands() {
    let ctx = WritingContext::new(WritingMode::VerticalLr, Direction::Ltr);
    let mut style = ComputedStyle::initial();

    // border-block: in vertical-lr, block-start is left, block-end is right
    style =
        logical_values::apply_with_context(style, ctx, "border-block", &tokens("3px solid black"))
            .unwrap();
    assert_eq!(style.border().left(), px(3.0));
    assert_eq!(style.border().right(), px(3.0));

    // border-inline-width: in vertical-lr ltr, inline-start is top, inline-end is bottom
    style =
        logical_values::apply_with_context(style, ctx, "border-inline-width", &tokens("1px 5px"))
            .unwrap();
    assert_eq!(style.border().top(), px(1.0));
    assert_eq!(style.border().bottom(), px(5.0));

    // border-inline: sets both inline sides
    style =
        logical_values::apply_with_context(style, ctx, "border-inline", &tokens("2px dashed red"))
            .unwrap();
    assert_eq!(style.border().top(), px(2.0));
    assert_eq!(style.border().bottom(), px(2.0));
}

// =========================================================================
// 7. Logical Insets Tests
// =========================================================================

#[test]
fn test_logical_insets() {
    let ctx = WritingContext::new(WritingMode::HorizontalTb, Direction::Ltr);
    let mut style = ComputedStyle::initial();

    style = logical_values::apply_with_context(style, ctx, "inset-inline-start", &tokens("10px"))
        .unwrap();
    style = logical_values::apply_with_context(style, ctx, "inset-inline-end", &tokens("20px"))
        .unwrap();
    style = logical_values::apply_with_context(style, ctx, "inset-block-start", &tokens("30px"))
        .unwrap();
    style =
        logical_values::apply_with_context(style, ctx, "inset-block-end", &tokens("40px")).unwrap();

    let pos = style.position();
    assert_eq!(
        pos.left(),
        css::domain::computed::sizing::Sizing::Fixed(px(10.0))
    );
    assert_eq!(
        pos.right(),
        css::domain::computed::sizing::Sizing::Fixed(px(20.0))
    );
    assert_eq!(
        pos.top(),
        css::domain::computed::sizing::Sizing::Fixed(px(30.0))
    );
    assert_eq!(
        pos.bottom(),
        css::domain::computed::sizing::Sizing::Fixed(px(40.0))
    );
}

#[test]
fn test_logical_inset_shorthands() {
    let ctx = WritingContext::new(WritingMode::VerticalRl, Direction::Rtl);
    let mut style = ComputedStyle::initial();

    // In vertical-rl rtl:
    // block-start -> right, block-end -> left
    // inline-start -> bottom, inline-end -> top
    style = logical_values::apply_with_context(style, ctx, "inset-block", &tokens("10px 20px"))
        .unwrap();
    style = logical_values::apply_with_context(style, ctx, "inset-inline", &tokens("30px 40px"))
        .unwrap();

    let pos = style.position();
    assert_eq!(
        pos.right(),
        css::domain::computed::sizing::Sizing::Fixed(px(10.0))
    );
    assert_eq!(
        pos.left(),
        css::domain::computed::sizing::Sizing::Fixed(px(20.0))
    );
    assert_eq!(
        pos.bottom(),
        css::domain::computed::sizing::Sizing::Fixed(px(30.0))
    );
    assert_eq!(
        pos.top(),
        css::domain::computed::sizing::Sizing::Fixed(px(40.0))
    );
}

// =========================================================================
// 8. Logical Style Accumulation & Parsing
// =========================================================================

#[test]
fn test_logical_style_lifecycle() {
    let mut logical = LogicalStyle::initial();

    assert!(logical_values::apply_to_logical_style(
        &mut logical,
        "writing-mode",
        &tokens("vertical-rl")
    ));
    assert!(logical_values::apply_to_logical_style(
        &mut logical,
        "direction",
        &tokens("rtl")
    ));
    assert!(logical_values::apply_to_logical_style(
        &mut logical,
        "inline-size",
        &tokens("300px")
    ));
    assert!(logical_values::apply_to_logical_style(
        &mut logical,
        "margin-inline",
        &tokens("10px 20px")
    ));
    assert!(logical_values::apply_to_logical_style(
        &mut logical,
        "padding-block",
        &tokens("5px")
    ));
    assert!(logical_values::apply_to_logical_style(
        &mut logical,
        "border-block",
        &tokens("2px solid black")
    ));
    assert!(logical_values::apply_to_logical_style(
        &mut logical,
        "inset-inline-start",
        &tokens("100px")
    ));

    let ctx = logical.context();
    assert_eq!(ctx.writing_mode(), WritingMode::VerticalRl);
    assert_eq!(ctx.direction(), Direction::Rtl);
    assert_eq!(
        logical.sizing().inline_size(),
        css::domain::computed::sizing::Sizing::Fixed(px(300.0))
    );
    assert_eq!(logical.margin().inline_start(), px(10.0));
    assert_eq!(logical.margin().inline_end(), px(20.0));
    assert_eq!(logical.padding().block_start(), px(5.0));
    assert_eq!(logical.border().block_start(), px(2.0));
    assert_eq!(
        logical.insets().inline_start(),
        css::domain::computed::sizing::Sizing::Fixed(px(100.0))
    );
}

#[test]
fn test_logical_edges_to_physical() {
    let ctx = WritingContext::new(WritingMode::HorizontalTb, Direction::Rtl);
    let edges = LogicalEdges::new(px(10.0), px(20.0), px(30.0), px(40.0));
    // In horizontal-tb rtl: block-start=top(10), block-end=bottom(30), inline-start=right(40), inline-end=left(20)
    let physical = edges.to_physical(ctx);
    assert_eq!(physical.top(), px(10.0));
    assert_eq!(physical.right(), px(40.0));
    assert_eq!(physical.bottom(), px(30.0));
    assert_eq!(physical.left(), px(20.0));
}

#[test]
fn test_logical_sizing_entity() {
    let mut sizing = LogicalSizing::initial();
    assert_eq!(
        sizing.inline_size(),
        css::domain::computed::sizing::Sizing::Auto
    );
    sizing = sizing.with_inline_size(css::domain::computed::sizing::Sizing::Fixed(px(100.0)));
    sizing = sizing.with_block_size(css::domain::computed::sizing::Sizing::Fixed(px(200.0)));
    sizing = sizing.with_min_inline_size(css::domain::computed::sizing::Sizing::Fixed(px(50.0)));
    sizing = sizing.with_min_block_size(css::domain::computed::sizing::Sizing::Fixed(px(60.0)));
    sizing = sizing.with_max_inline_size(css::domain::computed::sizing::Sizing::Fixed(px(300.0)));
    sizing = sizing.with_max_block_size(css::domain::computed::sizing::Sizing::Fixed(px(400.0)));

    assert_eq!(
        sizing.inline_size(),
        css::domain::computed::sizing::Sizing::Fixed(px(100.0))
    );
    assert_eq!(
        sizing.block_size(),
        css::domain::computed::sizing::Sizing::Fixed(px(200.0))
    );
    assert_eq!(
        sizing.min_inline_size(),
        css::domain::computed::sizing::Sizing::Fixed(px(50.0))
    );
    assert_eq!(
        sizing.min_block_size(),
        css::domain::computed::sizing::Sizing::Fixed(px(60.0))
    );
    assert_eq!(
        sizing.max_inline_size(),
        css::domain::computed::sizing::Sizing::Fixed(px(300.0))
    );
    assert_eq!(
        sizing.max_block_size(),
        css::domain::computed::sizing::Sizing::Fixed(px(400.0))
    );

    let ctx = WritingContext::new(WritingMode::HorizontalTb, Direction::Ltr);
    assert_eq!(
        LogicalSizing::inline_physical_axis(ctx),
        PhysicalAxis::Horizontal
    );
    assert_eq!(
        LogicalSizing::block_physical_axis(ctx),
        PhysicalAxis::Vertical
    );
}

#[test]
fn test_logical_insets_entity() {
    let insets = LogicalInsets::new(
        css::domain::computed::sizing::Sizing::Fixed(px(10.0)),
        css::domain::computed::sizing::Sizing::Fixed(px(20.0)),
        css::domain::computed::sizing::Sizing::Fixed(px(30.0)),
        css::domain::computed::sizing::Sizing::Fixed(px(40.0)),
    );
    assert_eq!(
        insets.block_start(),
        css::domain::computed::sizing::Sizing::Fixed(px(10.0))
    );
    assert_eq!(
        insets.block_end(),
        css::domain::computed::sizing::Sizing::Fixed(px(20.0))
    );
    assert_eq!(
        insets.inline_start(),
        css::domain::computed::sizing::Sizing::Fixed(px(30.0))
    );
    assert_eq!(
        insets.inline_end(),
        css::domain::computed::sizing::Sizing::Fixed(px(40.0))
    );

    let mut auto = LogicalInsets::AUTO;
    auto = auto.with_block_start(css::domain::computed::sizing::Sizing::Fixed(px(5.0)));
    auto = auto.with_block_end(css::domain::computed::sizing::Sizing::Fixed(px(15.0)));
    auto = auto.with_inline_start(css::domain::computed::sizing::Sizing::Fixed(px(25.0)));
    auto = auto.with_inline_end(css::domain::computed::sizing::Sizing::Fixed(px(35.0)));
    assert_eq!(
        auto.block_start(),
        css::domain::computed::sizing::Sizing::Fixed(px(5.0))
    );
    assert_eq!(
        auto.block_end(),
        css::domain::computed::sizing::Sizing::Fixed(px(15.0))
    );
    assert_eq!(
        auto.inline_start(),
        css::domain::computed::sizing::Sizing::Fixed(px(25.0))
    );
    assert_eq!(
        auto.inline_end(),
        css::domain::computed::sizing::Sizing::Fixed(px(35.0))
    );
}
