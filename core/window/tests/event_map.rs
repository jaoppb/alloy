//! `winit::event::WindowEvent` → `domain::WindowEvent` mapping totality
//! (`ADR-0011` item 2, `PRD-010`).
//!
//! Pure unit tests on `event_map::map_window_event` — no event loop, no
//! display server, so these run in ordinary CI. Only compiled under the
//! `winit-system` feature (the default): the `no-window` build links no
//! `winit` at all, so there is nothing here to test.
//!
//! `winit::event::KeyEvent` has no public constructor outside the `winit`
//! crate (`platform_specific` is `pub(crate)`), so a `KeyboardInput` event
//! cannot be built here. The adapter therefore splits the mapping at a
//! `KeyInput` — the five `KeyEvent` fields it reads, constructible in a test —
//! and these tests drive [`KeyboardState::translate`] through it, with the
//! values `winit` 0.30 reports on Linux (`xkb/mod.rs:317-319`: `text` ignores
//! Ctrl). The one line left uncovered is `KeyInput::from_winit`, a field copy.

#![cfg(feature = "winit-system")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use window::infrastructure::event_map::{map_cursor_icon, map_window_event};
use window::infrastructure::keyboard::{KeyInput, KeyboardState};
use window::{
    CursorIcon, InputText, KeyCode, KeyState, LogicalKey, PointerButton, SurfaceSize, WindowEvent,
};
use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState, NamedKey, PhysicalKey};

#[test]
fn resized_maps_to_the_same_dimensions() {
    let event = winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(800, 600));

    let mapped =
        map_window_event(event, &mut KeyboardState::default()).expect("Resized must map to Some");

    assert_eq!(
        mapped,
        WindowEvent::Resized(SurfaceSize::new(800, 600).expect("800x600 is valid"))
    );
}

#[test]
fn a_zero_sized_resize_is_dropped_rather_than_producing_an_invalid_surface_size() {
    let event = winit::event::WindowEvent::Resized(winit::dpi::PhysicalSize::new(0, 0));

    assert_eq!(
        map_window_event(event, &mut KeyboardState::default()),
        None,
        "a 0x0 surface has no SurfaceSize reading, so this must not fabricate one"
    );
}

#[test]
fn close_requested_maps_directly() {
    let event = winit::event::WindowEvent::CloseRequested;
    assert_eq!(
        map_window_event(event, &mut KeyboardState::default()),
        Some(WindowEvent::CloseRequested)
    );
}

#[test]
fn redraw_requested_maps_directly() {
    let event = winit::event::WindowEvent::RedrawRequested;
    assert_eq!(
        map_window_event(event, &mut KeyboardState::default()),
        Some(WindowEvent::RedrawRequested)
    );
}

#[test]
fn cursor_moved_maps_to_pointer_moved_with_the_same_coordinates() {
    let event = winit::event::WindowEvent::CursorMoved {
        device_id: winit::event::DeviceId::dummy(),
        position: winit::dpi::PhysicalPosition::new(12.5, 7.0),
    };

    let mapped = map_window_event(event, &mut KeyboardState::default())
        .expect("CursorMoved must map to Some");

    let WindowEvent::PointerMoved { position } = mapped else {
        panic!("expected PointerMoved, got {mapped:?}");
    };
    assert_eq!(position.x(), 12.5);
    assert_eq!(position.y(), 7.0);
}

#[test]
fn mouse_input_maps_button_and_pressed_state() {
    let event = winit::event::WindowEvent::MouseInput {
        device_id: winit::event::DeviceId::dummy(),
        state: winit::event::ElementState::Pressed,
        button: winit::event::MouseButton::Right,
    };

    let mapped = map_window_event(event, &mut KeyboardState::default())
        .expect("MouseInput must map to Some");

    assert_eq!(
        mapped,
        WindowEvent::PointerButton {
            button: PointerButton::Right,
            pressed: true,
        }
    );
}

#[test]
fn mouse_wheel_line_delta_maps_to_scroll() {
    let event = winit::event::WindowEvent::MouseWheel {
        device_id: winit::event::DeviceId::dummy(),
        delta: winit::event::MouseScrollDelta::LineDelta(1.0, -2.0),
        phase: winit::event::TouchPhase::Moved,
    };

    let mapped = map_window_event(event, &mut KeyboardState::default())
        .expect("MouseWheel must map to Some");

    let WindowEvent::Scroll { delta_x, delta_y } = mapped else {
        panic!("expected Scroll, got {mapped:?}");
    };
    assert_eq!(delta_x, 1.0);
    assert_eq!(delta_y, -2.0);
}

#[test]
fn mouse_wheel_pixel_delta_maps_to_scroll() {
    let event = winit::event::WindowEvent::MouseWheel {
        device_id: winit::event::DeviceId::dummy(),
        delta: winit::event::MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(
            3.0, 4.0,
        )),
        phase: winit::event::TouchPhase::Moved,
    };

    let mapped = map_window_event(event, &mut KeyboardState::default())
        .expect("MouseWheel must map to Some");

    let WindowEvent::Scroll { delta_x, delta_y } = mapped else {
        panic!("expected Scroll, got {mapped:?}");
    };
    assert_eq!(delta_x, 3.0);
    assert_eq!(delta_y, 4.0);
}

/// Every variant this port has decided **not** to represent yet must still be
/// named explicitly in `map_window_event` (compile-time totality) and produce
/// `None` here (behavioural confirmation it is a reviewed, not a silent, drop).
#[test]
fn variants_not_yet_represented_in_the_domain_vocabulary_map_to_none() {
    let dropped = [
        winit::event::WindowEvent::Moved(winit::dpi::PhysicalPosition::new(0, 0)),
        winit::event::WindowEvent::Destroyed,
        winit::event::WindowEvent::HoveredFileCancelled,
        winit::event::WindowEvent::Focused(true),
        winit::event::WindowEvent::CursorEntered {
            device_id: winit::event::DeviceId::dummy(),
        },
        winit::event::WindowEvent::CursorLeft {
            device_id: winit::event::DeviceId::dummy(),
        },
        winit::event::WindowEvent::Occluded(false),
    ];

    for event in dropped {
        assert_eq!(
            map_window_event(event.clone(), &mut KeyboardState::default()),
            None,
            "{event:?} is a deliberate, reviewed drop — it must map to None, not panic or map to Some"
        );
    }
}

#[test]
fn ime_commit_maps_to_text_committed() {
    let event = winit::event::WindowEvent::Ime(winit::event::Ime::Commit(String::from("Alloy")));
    let mapped = map_window_event(event, &mut KeyboardState::default())
        .expect("Ime::Commit must map to Some");

    assert_eq!(
        mapped,
        WindowEvent::TextCommitted(InputText::new("Alloy").unwrap())
    );
}

#[test]
fn an_ime_commit_with_nothing_printable_is_dropped() {
    for hostile in ["", "\r", "a\x08"] {
        let event = winit::event::WindowEvent::Ime(winit::event::Ime::Commit(hostile.into()));

        assert_eq!(
            map_window_event(event, &mut KeyboardState::default()),
            None,
            "{hostile:?} is not committable text"
        );
    }
}

#[test]
fn ime_non_commit_variants_map_to_none() {
    let dropped_ime = [
        winit::event::WindowEvent::Ime(winit::event::Ime::Enabled),
        winit::event::WindowEvent::Ime(winit::event::Ime::Preedit(
            String::from("composing"),
            Some((0, 9)),
        )),
        winit::event::WindowEvent::Ime(winit::event::Ime::Disabled),
    ];

    for event in dropped_ime {
        assert_eq!(
            map_window_event(event, &mut KeyboardState::default()),
            None,
            "non-commit IME events must map to None"
        );
    }
}

// ---- keyboard ----

const fn code(code: winit::keyboard::KeyCode) -> PhysicalKey {
    PhysicalKey::Code(code)
}

fn character(value: &str) -> Key {
    Key::Character(value.into())
}

fn input(physical: PhysicalKey, logical: Key, text: Option<&str>) -> KeyInput {
    KeyInput::new(physical, logical, text, ElementState::Pressed, false)
}

fn modifiers_changed(state: ModifiersState) -> winit::event::WindowEvent {
    winit::event::WindowEvent::ModifiersChanged(winit::event::Modifiers::from(state))
}

fn hold(keyboard: &mut KeyboardState, state: ModifiersState) {
    assert_eq!(
        map_window_event(modifiers_changed(state), keyboard),
        None,
        "ModifiersChanged updates the stamp and is not itself an event"
    );
}

fn text(value: &str) -> InputText {
    InputText::new(value).unwrap()
}

#[test]
fn alt_d_yields_one_key_event_carrying_alt_and_the_text_d() {
    let mut keyboard = KeyboardState::default();
    hold(&mut keyboard, ModifiersState::ALT);

    let event = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::KeyD),
        character("d"),
        Some("d"),
    ));

    assert!(event.modifiers().alt());
    assert!(!event.modifiers().control());
    assert_eq!(event.text(), Some(&text("d")));
    assert_eq!(event.logical(), &LogicalKey::Character(text("d")));
    assert_eq!(event.physical(), KeyCode::KEY_D);
    assert_eq!(event.state(), KeyState::Pressed);
}

#[test]
fn ctrl_l_carries_the_text_l_as_winit_reports_it_on_linux() {
    // `winit` `KeyEvent::text` ignores Ctrl on xkb (#117): the adapter must
    // pass that through untouched and let the keymap read the modifier.
    let mut keyboard = KeyboardState::default();
    hold(&mut keyboard, ModifiersState::CONTROL);

    let event = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::KeyL),
        character("l"),
        Some("l"),
    ));

    assert!(event.modifiers().control());
    assert_eq!(event.text(), Some(&text("l")));
    assert_eq!(event.physical(), KeyCode::KEY_L);
}

#[test]
fn modifiers_stamp_every_following_press_until_they_change_again() {
    let mut keyboard = KeyboardState::default();
    let letter = || {
        input(
            code(winit::keyboard::KeyCode::KeyA),
            character("a"),
            Some("a"),
        )
    };

    hold(&mut keyboard, ModifiersState::SHIFT | ModifiersState::SUPER);
    let held = keyboard.translate(&letter());
    hold(&mut keyboard, ModifiersState::empty());
    let released = keyboard.translate(&letter());

    assert!(held.modifiers().shift() && held.modifiers().super_key());
    assert!(released.modifiers().is_empty());
}

#[test]
fn a_modifier_pressed_before_the_window_had_focus_still_stamps_the_first_press() {
    // winit sends the current ModifiersChanged when focus arrives, with no
    // key event for the modifier itself — the case reconstruction from raw
    // key events cannot see (#112).
    let mut keyboard = KeyboardState::default();
    hold(&mut keyboard, ModifiersState::CONTROL);

    let event = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::KeyL),
        character("l"),
        Some("l"),
    ));

    assert!(event.modifiers().control());
}

#[test]
fn altgr_q_on_a_german_layout_yields_alt_graph_and_the_text_at_sign() {
    let mut keyboard = KeyboardState::default();
    let altgr = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::AltRight),
        Key::Named(NamedKey::AltGraph),
        None,
    ));

    let at_sign = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::KeyQ),
        character("@"),
        Some("@"),
    ));

    assert_eq!(altgr.logical(), &LogicalKey::AltGraph);
    assert!(at_sign.modifiers().alt_graph());
    assert!(
        !at_sign.modifiers().alt(),
        "winit reports no Alt bit for AltGr"
    );
    assert_eq!(at_sign.text(), Some(&text("@")));
    assert_eq!(at_sign.physical(), KeyCode::KEY_Q);
}

#[test]
fn releasing_altgr_stops_stamping_alt_graph() {
    let mut keyboard = KeyboardState::default();
    let altgr = |state| {
        KeyInput::new(
            code(winit::keyboard::KeyCode::AltRight),
            Key::Named(NamedKey::AltGraph),
            None,
            state,
            false,
        )
    };
    keyboard.translate(&altgr(ElementState::Pressed));
    keyboard.translate(&altgr(ElementState::Released));

    let after = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::KeyQ),
        character("q"),
        Some("q"),
    ));

    assert!(!after.modifiers().alt_graph());
}

#[test]
fn losing_focus_forgets_a_held_altgr() {
    // The release happens while another window has focus and never arrives.
    let mut keyboard = KeyboardState::default();
    keyboard.translate(&input(
        code(winit::keyboard::KeyCode::AltRight),
        Key::Named(NamedKey::AltGraph),
        None,
    ));

    let lost_focus = winit::event::WindowEvent::Focused(false);
    assert_eq!(map_window_event(lost_focus, &mut keyboard), None);

    let after = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::KeyQ),
        character("q"),
        Some("q"),
    ));
    assert!(!after.modifiers().alt_graph());
}

#[test]
fn a_held_key_yields_pressed_then_repeated_not_repeated_pressed() {
    let mut keyboard = KeyboardState::default();
    let step = |keyboard: &mut KeyboardState, state, repeat| {
        keyboard
            .translate(&KeyInput::new(
                code(winit::keyboard::KeyCode::KeyA),
                character("a"),
                Some("a"),
                state,
                repeat,
            ))
            .state()
    };

    let states = [
        step(&mut keyboard, ElementState::Pressed, false),
        step(&mut keyboard, ElementState::Pressed, true),
        step(&mut keyboard, ElementState::Pressed, true),
        step(&mut keyboard, ElementState::Released, false),
    ];

    assert_eq!(
        states,
        [
            KeyState::Pressed,
            KeyState::Repeated,
            KeyState::Repeated,
            KeyState::Released
        ]
    );
}

#[test]
fn a_release_carries_no_text() {
    let mut keyboard = KeyboardState::default();

    let event = keyboard.translate(&KeyInput::new(
        code(winit::keyboard::KeyCode::KeyA),
        character("a"),
        Some("a"),
        ElementState::Released,
        false,
    ));

    assert_eq!(event.text(), None);
}

#[test]
fn control_characters_in_the_text_are_dropped_but_the_key_is_kept() {
    let mut keyboard = KeyboardState::default();

    let enter = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::Enter),
        Key::Named(NamedKey::Enter),
        Some("\r"),
    ));
    let backspace = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::Backspace),
        Key::Named(NamedKey::Backspace),
        Some("\x08"),
    ));

    assert_eq!((enter.text(), enter.logical()), (None, &LogicalKey::Enter));
    assert_eq!(
        (backspace.text(), backspace.logical()),
        (None, &LogicalKey::Backspace)
    );
}

#[test]
fn space_carries_its_text() {
    let mut keyboard = KeyboardState::default();

    let event = keyboard.translate(&input(
        code(winit::keyboard::KeyCode::Space),
        Key::Named(NamedKey::Space),
        Some(" "),
    ));

    assert_eq!(event.logical(), &LogicalKey::Space);
    assert_eq!(event.text(), Some(&text(" ")));
}

#[test]
fn named_keys_map_to_their_logical_key() {
    let cases = [
        (NamedKey::Enter, LogicalKey::Enter),
        (NamedKey::Tab, LogicalKey::Tab),
        (NamedKey::Space, LogicalKey::Space),
        (NamedKey::Backspace, LogicalKey::Backspace),
        (NamedKey::Delete, LogicalKey::Delete),
        (NamedKey::Escape, LogicalKey::Escape),
        (NamedKey::Insert, LogicalKey::Insert),
        (NamedKey::ArrowUp, LogicalKey::ArrowUp),
        (NamedKey::ArrowDown, LogicalKey::ArrowDown),
        (NamedKey::ArrowLeft, LogicalKey::ArrowLeft),
        (NamedKey::ArrowRight, LogicalKey::ArrowRight),
        (NamedKey::Home, LogicalKey::Home),
        (NamedKey::End, LogicalKey::End),
        (NamedKey::PageUp, LogicalKey::PageUp),
        (NamedKey::PageDown, LogicalKey::PageDown),
        (NamedKey::F1, LogicalKey::F1),
        (NamedKey::F5, LogicalKey::F5),
        (NamedKey::F12, LogicalKey::F12),
        (NamedKey::Shift, LogicalKey::Shift),
        (NamedKey::Control, LogicalKey::Control),
        (NamedKey::Alt, LogicalKey::Alt),
        (NamedKey::AltGraph, LogicalKey::AltGraph),
        (NamedKey::Super, LogicalKey::Super),
        (NamedKey::CapsLock, LogicalKey::CapsLock),
        (NamedKey::Copy, LogicalKey::Other),
    ];

    for (named, expected) in cases {
        let mut keyboard = KeyboardState::default();
        let event = keyboard.translate(&input(
            PhysicalKey::Code(winit::keyboard::KeyCode::KeyA),
            Key::Named(named),
            None,
        ));
        assert_eq!(event.logical(), &expected, "{named:?}");
    }
}

#[test]
fn character_dead_and_unidentified_keys_map_to_their_logical_key() {
    let mut keyboard = KeyboardState::default();
    let logical_of = |keyboard: &mut KeyboardState, key| {
        keyboard
            .translate(&input(code(winit::keyboard::KeyCode::KeyA), key, None))
            .logical()
            .clone()
    };

    assert_eq!(
        logical_of(&mut keyboard, character("é")),
        LogicalKey::Character(text("é"))
    );
    assert_eq!(
        logical_of(&mut keyboard, Key::Dead(Some('´'))),
        LogicalKey::Dead
    );
    assert_eq!(
        logical_of(
            &mut keyboard,
            Key::Unidentified(winit::keyboard::NativeKey::Unidentified)
        ),
        LogicalKey::Unidentified
    );
    assert_eq!(
        logical_of(&mut keyboard, character("\u{7f}")),
        LogicalKey::Unidentified,
        "a character key that is a control character has no printable meaning"
    );
}

#[test]
fn an_unidentified_physical_key_maps_to_the_unidentified_key_code() {
    let mut keyboard = KeyboardState::default();

    let event = keyboard.translate(&input(
        PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
        character("a"),
        Some("a"),
    ));

    assert_eq!(event.physical(), KeyCode::UNIDENTIFIED);
}

// ---- cursor ----

#[test]
fn cursor_icons_map_to_winit_cursor_icons() {
    let cases = [
        (CursorIcon::Default, winit::window::CursorIcon::Default),
        (CursorIcon::Pointer, winit::window::CursorIcon::Pointer),
        (CursorIcon::Text, winit::window::CursorIcon::Text),
        (CursorIcon::Wait, winit::window::CursorIcon::Wait),
        (CursorIcon::Progress, winit::window::CursorIcon::Progress),
        (CursorIcon::Crosshair, winit::window::CursorIcon::Crosshair),
        (
            CursorIcon::NotAllowed,
            winit::window::CursorIcon::NotAllowed,
        ),
    ];

    for (icon, expected) in cases {
        assert_eq!(map_cursor_icon(icon), expected, "{icon:?}");
    }
}
