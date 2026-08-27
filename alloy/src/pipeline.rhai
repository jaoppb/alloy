// Alloy Full Rendering Pipeline Orchestration (Rhai Module)
// Paired with alloy/src/main.rs

fn run_pipeline(viewport_width, viewport_height) {
    // 1. Clear background surface
    renderer.clear("white");

    // 2. Draw Browser Shell Header
    renderer.pushRect(0.0, 0.0, viewport_width, 42.0, "#2b2d42");
    renderer.pushText("Alloy Browser v0.1.0-alpha", 16.0, 12.0, 300.0, 20.0, 14.0, "white");

    // 3. Draw Content Frame Container
    let card_x = 24.0;
    let card_y = 66.0;
    let card_w = viewport_width - 48.0;
    let card_h = viewport_height - 90.0;

    renderer.pushRect(card_x, card_y, card_w, card_h, "#ffffff");
    renderer.pushBorder(card_x, card_y, card_w, card_h, 1.5, "#8d99ae");

    // 4. Render Headless Document Mockup
    renderer.pushText("Hello from Alloy Engine!", card_x + 24.0, card_y + 24.0, 400.0, 32.0, 24.0, "#1d3557");
    renderer.pushText("A modular multi-process browser built in Rust and scriptable with Rhai.", card_x + 24.0, card_y + 68.0, card_w - 48.0, 20.0, 14.0, "#457b9d");

    // 5. Draw Status Badge
    let badge_y = card_y + 110.0;
    renderer.pushRect(card_x + 24.0, badge_y, 140.0, 32.0, "#2a9d8f");
    renderer.pushText("PIPELINE OK", card_x + 36.0, badge_y + 7.0, 116.0, 18.0, 13.0, "white");
}
