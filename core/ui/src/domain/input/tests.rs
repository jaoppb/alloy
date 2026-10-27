#![allow(clippy::unwrap_used)]

use window::{InputText, KeyCode, KeyEvent, KeyState, LogicalKey, Modifiers};

use super::{ActionName, BrowserCommand, ChordKey, KeyChord, Keymap, KeymapBinding, KeymapError};

fn character(value: &str) -> LogicalKey {
    LogicalKey::Character(InputText::new(value).unwrap())
}

fn letter(modifiers: Modifiers, value: &str) -> KeyChord {
    KeyChord::new(modifiers, ChordKey::Logical(character(value)))
}

fn press(logical: LogicalKey, physical: KeyCode, modifiers: Modifiers) -> KeyEvent {
    KeyEvent::new(logical, physical, KeyState::Pressed).with_modifiers(modifiers)
}

#[test]
fn default_keymap_binds_the_focus_shortcuts() {
    let keymap = Keymap::default();
    let ctrl = Modifiers::NONE.with_control();
    let super_key = Modifiers::NONE.with_super_key();
    let alt = Modifiers::NONE.with_alt();
    for event in [
        press(character("l"), KeyCode::KEY_L, ctrl),
        press(character("l"), KeyCode::KEY_L, super_key),
        press(character("d"), KeyCode::KEY_D, alt),
    ] {
        assert_eq!(keymap.resolve(&event), Some(BrowserCommand::FocusOmnibox));
    }
}

#[test]
fn chords_match_the_physical_key_when_the_logical_key_is_not_latin() {
    let russian_ctrl_l = press(
        character("д"),
        KeyCode::KEY_L,
        Modifiers::NONE.with_control(),
    );
    assert_eq!(
        Keymap::default().resolve(&russian_ctrl_l),
        Some(BrowserCommand::FocusOmnibox)
    );
}

#[test]
fn chords_match_modifiers_exactly_and_ignore_character_case() {
    let keymap = Keymap::default();
    let shifted = Modifiers::NONE.with_control().with_shift();
    assert_eq!(
        keymap.resolve(&press(character("L"), KeyCode::UNIDENTIFIED, shifted)),
        None
    );
    let caps_lock_ctrl_l = press(
        character("L"),
        KeyCode::UNIDENTIFIED,
        Modifiers::NONE.with_control(),
    );
    assert_eq!(
        keymap.resolve(&caps_lock_ctrl_l),
        Some(BrowserCommand::FocusOmnibox)
    );
}

#[test]
fn chord_text_parses_modifiers_and_keys() {
    let chord: KeyChord = "Ctrl+Shift+ArrowLeft".parse().unwrap();
    let expected = KeyChord::new(
        Modifiers::NONE.with_control().with_shift(),
        ChordKey::Logical(LogicalKey::ArrowLeft),
    );
    assert_eq!(chord, expected);
    let plus: KeyChord = "ctrl++".parse().unwrap();
    assert_eq!(plus, letter(Modifiers::NONE.with_control(), "+"));
    let alt_graph: KeyChord = "ctrl+altgr+l".parse().unwrap();
    let expected = letter(Modifiers::NONE.with_control().with_alt_graph(), "l");
    assert_eq!(alt_graph, expected);
}

#[test]
fn malformed_chords_are_typed_errors() {
    assert!(matches!(
        "".parse::<KeyChord>(),
        Err(KeymapError::MissingKey(_))
    ));
    assert!(matches!(
        "ctrl+".parse::<KeyChord>(),
        Err(KeymapError::MissingKey(_))
    ));
    assert!(matches!(
        "hyper+l".parse::<KeyChord>(),
        Err(KeymapError::UnknownModifier { .. })
    ));
    assert!(matches!(
        "ctrl+ll".parse::<KeyChord>(),
        Err(KeymapError::UnknownKey { .. })
    ));
}

#[test]
fn command_names_round_trip_and_unknown_names_are_typed_errors() {
    let command: BrowserCommand = "focus_omnibox".parse().unwrap();
    assert_eq!(command.to_string(), "focus_omnibox");
    assert_eq!(
        "reload_everything".parse::<BrowserCommand>(),
        Err(KeymapError::UnknownAction(ActionName::from(
            "reload_everything"
        )))
    );
}

#[test]
fn from_bindings_keeps_valid_bindings_and_reports_the_rest() {
    let (keymap, rejected) = Keymap::from_bindings([
        KeymapBinding::new("ctrl+k", "focus_omnibox"),
        KeymapBinding::new("ctrl+", "focus_omnibox"),
        KeymapBinding::new("ctrl+j", "teleport"),
    ]);
    let ctrl = Modifiers::NONE.with_control();
    let ctrl_k = press(character("k"), KeyCode::KEY_K, ctrl);
    assert_eq!(keymap.resolve(&ctrl_k), Some(BrowserCommand::FocusOmnibox));
    assert_eq!(rejected.len(), 2);
    let ctrl_l = press(character("l"), KeyCode::KEY_L, ctrl);
    assert_eq!(
        keymap.resolve(&ctrl_l),
        None,
        "a valid set replaces the defaults"
    );
}

#[test]
fn an_entirely_invalid_set_keeps_the_default_keymap() {
    let (keymap, rejected) = Keymap::from_bindings([KeymapBinding::new("nope+", "nothing")]);
    assert_eq!(keymap, Keymap::default());
    assert_eq!(rejected.len(), 1);
    assert_eq!(Keymap::from_bindings([]).0, Keymap::default());
}

#[test]
fn positional_chords_bind_the_key_at_a_position() {
    let chord: KeyChord = "ctrl+code:l".parse().unwrap();
    let expected = KeyChord::new(
        Modifiers::NONE.with_control(),
        ChordKey::Physical(KeyCode::KEY_L),
    );
    assert_eq!(chord, expected);
    let digit: KeyChord = "alt+code:7".parse().unwrap();
    assert_eq!(
        digit,
        KeyChord::new(
            Modifiers::NONE.with_alt(),
            ChordKey::Physical(KeyCode::DIGIT7)
        )
    );
    assert!(matches!(
        "ctrl+code:".parse::<KeyChord>(),
        Err(KeymapError::UnknownKey { .. })
    ));
    assert!(matches!(
        "ctrl+code:ll".parse::<KeyChord>(),
        Err(KeymapError::UnknownKey { .. })
    ));
    assert!(matches!(
        "ctrl+code:!".parse::<KeyChord>(),
        Err(KeymapError::UnknownKey { .. })
    ));
}

#[test]
fn the_default_keymap_is_built_from_valid_binding_text() {
    let defaults = [
        "ctrl+l",
        "super+l",
        "alt+d",
        "ctrl+code:l",
        "super+code:l",
        "alt+code:d",
    ]
    .map(|chord| KeymapBinding::new(chord, "focus_omnibox"));
    let (keymap, rejected) = Keymap::from_bindings(defaults);
    assert!(rejected.is_empty());
    assert_eq!(keymap, Keymap::default());
}
