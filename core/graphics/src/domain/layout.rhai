// Alloy 2D Layout Engine (Rhai Module)
// Paired with core/graphics/src/domain/layout.rs

fn compute_box_bounds(x, y, width, height, margin_top, margin_left) {
    return #{
        x: x + margin_left,
        y: y + margin_top,
        width: width,
        height: height
    };
}

fn compute_content_inner(x, y, width, height, padding_top, padding_left, padding_right, padding_bottom) {
    return #{
        x: x + padding_left,
        y: y + padding_top,
        width: width - padding_left - padding_right,
        height: height - padding_top - padding_bottom
    };
}
