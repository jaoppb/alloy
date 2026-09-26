// Default UI policy script (Fase M, ADR-0003, PRD-003:58).
// Runs under WINDOW_MANAGE | GRAPHICS_DRAW | DOM_READ.

fn on_init() {
    title("Alloy");
    repaint();
}

fn on_event(event) {
    if event == "resize" {
        repaint();
    } else if event == "key_reload" {
        key_shortcut("ctrl+r", "reload");
    } else {
        route("window", event);
    }
}

fn on_process(state) {
    // Idle loop processing
}

// Top-level evaluation fallback
if variable::is_bound("event") {
    on_event(event);
} else {
    on_init();
}
