//! Unit coverage for the v4 keyboard vocabulary in `domain/` — [`InputText`],
//! [`Modifiers`], [`KeyState`], [`LogicalKey`], [`KeyEvent`] and
//! [`CursorIcon`] (`PRD-010` §6, `ADR-0027`). No adapter involved: runs
//! identically under `--no-default-features` and the default build.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use window::{
    CursorIcon, FunctionKey, InputText, KeyCode, KeyEvent, KeyState, LogicalKey, Modifiers,
    WindowEvent,
};

fn text(value: &str) -> InputText {
    InputText::new(value).expect("a printable fixture is valid input text")
}

// ---- InputText ----

#[test]
fn input_text_refuses_the_empty_string() {
    assert_eq!(InputText::new(""), None);
}

#[test]
fn input_text_refuses_every_control_character() {
    for hostile in ["\r", "\n", "\t", "\x08", "\x1b", "\x7f", "ab\x0c", "\u{85}"] {
        assert_eq!(
            InputText::new(hostile),
            None,
            "{hostile:?} contains a control character"
        );
    }
}

#[test]
fn input_text_keeps_printable_text_verbatim() {
    for printable in ["a", " ", "Alloy 123", "café", "日本語", "🦀", "@"] {
        assert_eq!(InputText::new(printable).unwrap().as_str(), printable);
    }
}

#[test]
fn input_text_prints_verbatim() {
    assert_eq!(text("olá").to_string(), "olá");
}

// ---- Modifiers ----

#[test]
fn no_modifiers_reports_nothing_held() {
    let none = Modifiers::NONE;

    assert!(!none.shift());
    assert!(!none.control());
    assert!(!none.alt());
    assert!(!none.alt_graph());
    assert!(!none.super_key());
    assert!(none.is_empty());
}

#[test]
fn each_modifier_is_reported_independently() {
    let held = Modifiers::NONE.with_control().with_alt_graph();

    assert!(held.control());
    assert!(held.alt_graph());
    assert!(!held.shift());
    assert!(!held.alt());
    assert!(!held.super_key());
    assert!(!held.is_empty());
}

#[test]
fn alt_and_alt_graph_are_distinct_modifiers() {
    assert_ne!(Modifiers::NONE.with_alt(), Modifiers::NONE.with_alt_graph());
    assert!(!Modifiers::NONE.with_alt().alt_graph());
    assert!(!Modifiers::NONE.with_alt_graph().alt());
}

#[test]
fn adding_a_modifier_twice_changes_nothing() {
    let once = Modifiers::NONE.with_shift();

    assert_eq!(once.with_shift(), once);
}

// ---- KeyState ----

#[test]
fn key_state_distinguishes_a_fresh_press_from_a_repeat_and_a_release() {
    assert!(KeyState::Pressed.is_pressed());
    assert!(!KeyState::Repeated.is_pressed());
    assert!(!KeyState::Released.is_pressed());

    assert!(KeyState::Released.is_released());
    assert!(!KeyState::Pressed.is_released());
    assert!(!KeyState::Repeated.is_released());
}

#[test]
fn key_state_prints_a_lowercase_name() {
    assert_eq!(KeyState::Pressed.to_string(), "pressed");
    assert_eq!(KeyState::Repeated.to_string(), "repeated");
    assert_eq!(KeyState::Released.to_string(), "released");
}

const fn function(number: u8) -> LogicalKey {
    LogicalKey::Function(FunctionKey::new(number).expect("F1..=F12 are valid function keys"))
}

// ---- FunctionKey ----

#[test]
fn a_function_key_is_one_of_f1_to_f12() {
    for number in 1..=12 {
        assert_eq!(FunctionKey::new(number).unwrap().number(), number);
    }
}

#[test]
fn a_function_key_outside_f1_to_f12_has_no_reading() {
    assert_eq!(FunctionKey::new(0), None);
    assert_eq!(FunctionKey::new(13), None);
}

// ---- LogicalKey ----

#[test]
fn a_character_key_carries_the_character_it_produces() {
    assert_eq!(
        LogicalKey::Character(text("a")),
        LogicalKey::Character(text("a"))
    );
    assert_ne!(
        LogicalKey::Character(text("a")),
        LogicalKey::Character(text("A"))
    );
    assert_ne!(LogicalKey::Character(text("a")), LogicalKey::Enter);
}

#[test]
fn logical_key_display_matches_name() {
    assert_eq!(LogicalKey::Enter.to_string(), "Enter");
    assert_eq!(LogicalKey::Tab.to_string(), "Tab");
    assert_eq!(LogicalKey::Space.to_string(), "Space");
    assert_eq!(LogicalKey::Backspace.to_string(), "Backspace");
    assert_eq!(LogicalKey::Delete.to_string(), "Delete");
    assert_eq!(LogicalKey::Escape.to_string(), "Escape");
    assert_eq!(LogicalKey::ArrowUp.to_string(), "ArrowUp");
    assert_eq!(LogicalKey::ArrowDown.to_string(), "ArrowDown");
    assert_eq!(LogicalKey::ArrowLeft.to_string(), "ArrowLeft");
    assert_eq!(LogicalKey::ArrowRight.to_string(), "ArrowRight");
    assert_eq!(LogicalKey::Home.to_string(), "Home");
    assert_eq!(LogicalKey::End.to_string(), "End");
    assert_eq!(LogicalKey::PageUp.to_string(), "PageUp");
    assert_eq!(LogicalKey::PageDown.to_string(), "PageDown");
    assert_eq!(LogicalKey::AltGraph.to_string(), "AltGraph");
    assert_eq!(function(5).to_string(), "F5");
    assert_eq!(function(12).to_string(), "F12");
    assert_eq!(LogicalKey::Character(text("é")).to_string(), "é");
}

// ---- KeyEvent ----

#[test]
fn a_key_event_without_text_or_modifiers_carries_only_what_it_was_built_with() {
    let event = KeyEvent::new(LogicalKey::Enter, KeyCode::ENTER, KeyState::Pressed);

    assert_eq!(event.logical(), &LogicalKey::Enter);
    assert_eq!(event.physical(), KeyCode::ENTER);
    assert_eq!(event.state(), KeyState::Pressed);
    assert_eq!(event.text(), None);
    assert!(event.modifiers().is_empty());
}

#[test]
fn a_key_event_carries_the_text_and_modifiers_of_the_same_press() {
    let event = KeyEvent::new(
        LogicalKey::Character(text("d")),
        KeyCode::KEY_D,
        KeyState::Pressed,
    )
    .with_text(text("d"))
    .with_modifiers(Modifiers::NONE.with_alt());

    assert_eq!(event.text().map(InputText::as_str), Some("d"));
    assert!(event.modifiers().alt());
    assert_eq!(event.physical(), KeyCode::KEY_D);
}

#[test]
fn a_key_event_and_a_committed_text_are_distinct_window_events() {
    let key = WindowEvent::Key(KeyEvent::new(
        LogicalKey::Character(text("a")),
        KeyCode::KEY_A,
        KeyState::Pressed,
    ));
    let committed = WindowEvent::TextCommitted(text("a"));

    assert_ne!(key, committed);
}

// ---- CursorIcon ----

#[test]
fn the_default_cursor_icon_is_the_platform_default() {
    assert_eq!(CursorIcon::default(), CursorIcon::Default);
    assert_ne!(CursorIcon::Pointer, CursorIcon::Default);
}
