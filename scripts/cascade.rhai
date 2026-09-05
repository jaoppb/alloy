// Cascade policy override script (Fase M, ADR-0003, PRD-007 §3.4).
// Runs under DOM_READ | GRAPHICS_DRAW (strictly without DOM_MUTATE).

fn on_init() {
    // Cascade policy initialization
}

// Inspect DOM snapshot and apply computed styling overrides
let total = dom.len();
let i = 0;
while i < total {
    let tag_name = dom.tag(i);
    if tag_name == "h1" {
        tree.set_color(i, "#ff0000");
    }
    i += 1;
}
