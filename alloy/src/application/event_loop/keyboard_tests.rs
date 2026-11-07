//! Omnibox keyboard behaviour through the event loop, with events shaped the
//! way `winit` really delivers them (#117, `ADR-0027`).
#![allow(clippy::unwrap_used)]

use window::{
    HeadlessWindowSystem, InputText, KeyCode, KeyEvent, KeyState, LogicalKey, Modifiers,
    RecordingPresenter, WindowEvent, WindowSystem as _,
};

use crate::application::test_support::{
    FakeWeb, TestApp, app_over, completed_navigation, load, loaded_window, pump,
};
use crate::composition::initial_window_attributes;

// ---- keyboard fixtures ----
//
// Events are shaped the way `winit` really delivers them on Linux (#117):
// `KeyEvent::text` ignores Ctrl, so Ctrl+L arrives as logical `l` + text
// `"l"` + the Control modifier, and Alt+D as text `"d"` + the Alt modifier.

fn input_text(value: &str) -> InputText {
    InputText::new(value).unwrap()
}

/// A letter pressed and released with `modifiers` held; only the press carries
/// the text `winit` reports.
fn chord(system: &mut HeadlessWindowSystem, modifiers: Modifiers, physical: KeyCode, letter: &str) {
    let key = LogicalKey::Character(input_text(letter));
    let pressed = KeyEvent::new(key.clone(), physical, KeyState::Pressed)
        .with_modifiers(modifiers)
        .with_text(input_text(letter));
    let released = KeyEvent::new(key, physical, KeyState::Released).with_modifiers(modifiers);
    system.schedule(WindowEvent::Key(pressed));
    system.schedule(WindowEvent::Key(released));
}

fn ctrl_l(system: &mut HeadlessWindowSystem) {
    chord(system, Modifiers::NONE.with_control(), KeyCode::KEY_L, "l");
}

fn alt_d(system: &mut HeadlessWindowSystem) {
    chord(system, Modifiers::NONE.with_alt(), KeyCode::KEY_D, "d");
}

/// Plain typing: one unmodified key press per character.
fn type_text(system: &mut HeadlessWindowSystem, text: &str) {
    for character in text.chars() {
        let character = character.to_string();
        let event = KeyEvent::new(
            LogicalKey::Character(input_text(&character)),
            KeyCode::UNIDENTIFIED,
            KeyState::Pressed,
        )
        .with_text(input_text(&character));
        system.schedule(WindowEvent::Key(event));
    }
}

fn press_named(system: &mut HeadlessWindowSystem, logical: LogicalKey, physical: KeyCode) {
    system.schedule(WindowEvent::Key(KeyEvent::new(
        logical,
        physical,
        KeyState::Pressed,
    )));
}

/// Presses one keyboard shortcut on the window.
type Shortcut = fn(&mut HeadlessWindowSystem);

fn super_l(system: &mut HeadlessWindowSystem) {
    chord(
        system,
        Modifiers::NONE.with_super_key(),
        KeyCode::KEY_L,
        "l",
    );
}

#[test]
fn focus_shortcuts_focus_the_omnibox_and_select_all_text() {
    let shortcuts: [(&str, Shortcut); 3] =
        [("Ctrl+L", ctrl_l), ("Alt+D", alt_d), ("Super+L", super_l)];

    for (name, press) in shortcuts {
        let (mut system, mut presenter, mut app) = loaded_window();
        pump(&mut system, &mut presenter, &mut app);
        assert!(!app.ui().omnibox().is_focused());
        assert!(!app.ui().omnibox().is_all_selected());

        press(&mut system);
        pump(&mut system, &mut presenter, &mut app);

        assert!(
            app.ui().omnibox().is_focused(),
            "{name} must focus the omnibox"
        );
        assert!(
            app.ui().omnibox().is_all_selected(),
            "{name} must select existing omnibox text"
        );
    }
}

#[test]
fn typing_when_focused_appends_to_buffer_and_requests_redraw_without_relayout() {
    let (mut system, mut presenter, mut app) = loaded_window();

    // Initial pump: document relayouts once.
    pump(&mut system, &mut presenter, &mut app);
    let relayouts_before = app.stats().relayouts();
    assert_eq!(relayouts_before, 1);

    // Focus omnibox via Ctrl+L.
    ctrl_l(&mut system);
    pump(&mut system, &mut presenter, &mut app);
    assert!(app.ui().omnibox().is_focused());
    assert_eq!(
        app.stats().relayouts(),
        relayouts_before,
        "focusing omnibox must not trigger a DOM relayout"
    );

    // First character replaces selected text.
    type_text(&mut system, "h");
    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(app.ui().omnibox().text(), "h");
    assert_eq!(
        app.stats().relayouts(),
        relayouts_before,
        "typing in omnibox must strictly leave stats.relayouts unchanged"
    );

    // Subsequent characters append to the buffer.
    type_text(&mut system, "ello");
    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(app.ui().omnibox().text(), "hello");
    assert_eq!(
        app.stats().relayouts(),
        relayouts_before,
        "typing in omnibox must strictly leave stats.relayouts unchanged"
    );
}

#[test]
fn typing_when_unfocused_is_ignored() {
    let (mut system, mut presenter, mut app) = loaded_window();

    pump(&mut system, &mut presenter, &mut app);
    let initial_text = app.ui().omnibox().text().to_string();

    type_text(&mut system, "ignored");
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        app.ui().omnibox().text(),
        initial_text.as_str(),
        "typing without omnibox focus must not mutate the buffer"
    );
}

/// A loaded page whose omnibox is focused and holds `"abc"`, reached with the
/// Ctrl+L `winit` really sends.
fn focused_omnibox_holding_abc() -> (HeadlessWindowSystem, RecordingPresenter, TestApp) {
    let (mut system, mut presenter, mut app) = loaded_window();
    pump(&mut system, &mut presenter, &mut app);
    ctrl_l(&mut system);
    type_text(&mut system, "abc");
    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(app.ui().omnibox().text(), "abc");
    (system, presenter, app)
}

#[test]
fn an_unbound_ctrl_chord_inserts_nothing_even_though_winit_reports_its_letter_as_text() {
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();

    chord(
        &mut system,
        Modifiers::NONE.with_control(),
        KeyCode::KEY_K,
        "k",
    );
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        app.ui().omnibox().text(),
        "abc",
        "Ctrl+K carries text \"k\" on Linux; it must not be typed"
    );
}

#[test]
fn an_editing_key_under_a_command_modifier_is_dropped() {
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();
    let backspace_with = |modifiers| {
        WindowEvent::Key(
            KeyEvent::new(LogicalKey::Backspace, KeyCode::BACKSPACE, KeyState::Pressed)
                .with_modifiers(modifiers),
        )
    };

    system.schedule(backspace_with(Modifiers::NONE.with_control()));
    system.schedule(backspace_with(Modifiers::NONE.with_alt()));
    system.schedule(backspace_with(Modifiers::NONE.with_super_key()));
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(
        app.ui().omnibox().text(),
        "abc",
        "Ctrl/Alt/Super+Backspace are chords for the keymap, not edits"
    );
}

#[test]
fn alt_d_refocuses_and_selects_instead_of_typing_its_letter() {
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();

    alt_d(&mut system);
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(app.ui().omnibox().text(), "abc");
    assert!(app.ui().omnibox().is_all_selected());
}

#[test]
fn a_plain_l_types_a_letter_and_is_not_a_shortcut() {
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();

    chord(&mut system, Modifiers::NONE, KeyCode::KEY_L, "l");
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(app.ui().omnibox().text(), "abcl");
}

#[test]
fn shift_does_not_stop_a_letter_from_typing() {
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();

    chord(
        &mut system,
        Modifiers::NONE.with_shift(),
        KeyCode::KEY_D,
        "D",
    );
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(app.ui().omnibox().text(), "abcD");
}

#[test]
fn altgr_q_on_a_german_layout_types_the_at_sign() {
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();

    chord(
        &mut system,
        Modifiers::NONE.with_alt_graph(),
        KeyCode::KEY_Q,
        "@",
    );
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(app.ui().omnibox().text(), "abc@");
}

#[test]
fn altgr_with_a_command_modifier_still_types_when_no_binding_matches() {
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();

    chord(
        &mut system,
        Modifiers::NONE.with_alt_graph().with_control(),
        KeyCode::KEY_Q,
        "@",
    );
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(app.ui().omnibox().text(), "abc@");
}

#[test]
fn the_default_keymap_binds_no_altgr_chord_so_ctrl_altgr_l_types() {
    // Chords match modifiers exactly (`ui::KeyChord`), so the default Ctrl+L
    // does not fire with AltGr also held. A keymap that binds Ctrl+AltGr+L wins
    // over the AltGr insert: pinned by `ui`'s router tests (ADR-0027 step 1).
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();

    chord(
        &mut system,
        Modifiers::NONE.with_alt_graph().with_control(),
        KeyCode::KEY_L,
        "l",
    );
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(app.ui().omnibox().text(), "abcl");
}

#[test]
fn text_committed_by_an_input_method_is_inserted_into_the_focused_omnibox() {
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();

    system.schedule(WindowEvent::TextCommitted(input_text("日本語")));
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(app.ui().omnibox().text(), "abc日本語");
}

#[test]
fn a_held_backspace_keeps_deleting_but_its_release_does_nothing() {
    let (mut system, mut presenter, mut app) = focused_omnibox_holding_abc();
    let backspace = |state| {
        WindowEvent::Key(KeyEvent::new(
            LogicalKey::Backspace,
            KeyCode::BACKSPACE,
            state,
        ))
    };

    system.schedule(backspace(KeyState::Pressed));
    system.schedule(backspace(KeyState::Repeated));
    system.schedule(backspace(KeyState::Released));
    pump(&mut system, &mut presenter, &mut app);

    assert_eq!(app.ui().omnibox().text(), "a");
}

#[test]
fn a_shortcut_fires_on_the_press_not_on_every_auto_repeat() {
    let (mut system, mut presenter, mut app) = loaded_window();
    pump(&mut system, &mut presenter, &mut app);
    let repeated_ctrl_l = KeyEvent::new(
        LogicalKey::Character(input_text("l")),
        KeyCode::KEY_L,
        KeyState::Repeated,
    )
    .with_modifiers(Modifiers::NONE.with_control())
    .with_text(input_text("l"));

    system.schedule(WindowEvent::Key(repeated_ctrl_l));
    pump(&mut system, &mut presenter, &mut app);

    assert!(!app.ui().omnibox().is_focused());
}

#[test]
fn escape_cancels_editing_reverts_to_active_url_and_unfocuses() {
    let (mut system, mut presenter, mut app) = loaded_window();

    pump(&mut system, &mut presenter, &mut app);
    let original_url_text = app.ui().omnibox().text().to_string();

    // Focus and type.
    ctrl_l(&mut system);
    pump(&mut system, &mut presenter, &mut app);

    type_text(&mut system, "draft query");
    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(app.ui().omnibox().text(), "draft query");
    assert!(app.ui().omnibox().is_focused());

    // Press Escape.
    press_named(&mut system, LogicalKey::Escape, KeyCode::ESCAPE);
    pump(&mut system, &mut presenter, &mut app);

    assert!(
        !app.ui().omnibox().is_focused(),
        "Escape must release omnibox focus"
    );
    assert_eq!(
        app.ui().omnibox().text(),
        original_url_text.as_str(),
        "Escape must revert omnibox text to active page URL"
    );
}

#[test]
fn end_to_end_shortcut_typing_and_navigation_dispatch() {
    let attributes = initial_window_attributes().unwrap();
    let mut system = HeadlessWindowSystem::new();
    system.create_window(&attributes).unwrap();
    let mut presenter = RecordingPresenter::new();

    let target = network::Url::parse("https://rust-lang.org/").unwrap();
    let mut app = app_over(
        FakeWeb::new().with_page(target.clone(), "<html><body>Rust</body></html>"),
        attributes.initial_size(),
    );
    load(
        &mut app,
        "<html><body>Initial page</body></html>",
        "http://example.com/index.html",
    );
    pump(&mut system, &mut presenter, &mut app);
    let navigations_before = app.stats().navigations();

    // 1. Press Ctrl+L to focus omnibox.
    ctrl_l(&mut system);
    pump(&mut system, &mut presenter, &mut app);
    assert!(app.ui().omnibox().is_focused());

    // 2. Type "rust-lang.org" (without scheme).
    type_text(&mut system, "rust-lang.org");
    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(app.ui().omnibox().text(), "rust-lang.org");

    // 3. Press Enter.
    press_named(&mut system, LogicalKey::Enter, KeyCode::ENTER);
    pump(&mut system, &mut presenter, &mut app);

    // Focus must be released immediately.
    assert!(
        !app.ui().omnibox().is_focused(),
        "pressing Enter must release omnibox focus"
    );

    // Navigation must be classified as https://rust-lang.org/ and completed.
    assert_eq!(
        completed_navigation(&mut system, &mut presenter, &mut app, navigations_before),
        target,
        "Enter must classify input and dispatch navigation to classified destination"
    );
}

#[test]
fn control_keys_backspace_and_arrows_operate_on_focused_omnibox() {
    let (mut system, mut presenter, mut app) = loaded_window();

    pump(&mut system, &mut presenter, &mut app);

    // Focus via Ctrl+L and type "cat"
    ctrl_l(&mut system);
    type_text(&mut system, "cat");
    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(app.ui().omnibox().text(), "cat");

    // Backspace removes 't' -> "ca"
    press_named(&mut system, LogicalKey::Backspace, KeyCode::BACKSPACE);
    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(app.ui().omnibox().text(), "ca");

    // ArrowLeft moves cursor between 'c' and 'a', then type 'o' -> "coa"
    press_named(&mut system, LogicalKey::ArrowLeft, KeyCode::ARROW_LEFT);
    type_text(&mut system, "o");
    pump(&mut system, &mut presenter, &mut app);
    assert_eq!(app.ui().omnibox().text(), "coa");
}
