#![allow(clippy::unwrap_used)]

use window::{InputText, KeyCode, KeyEvent, KeyState, LogicalKey, Modifiers};

use super::{BrowserCommand, ChordKey, KeyChord, Keymap, KeymapBinding, KeymapError, UiInput};
use crate::domain::input::FocusTarget;
use crate::domain::omnibox::EditKey;

fn character(value: &str) -> LogicalKey {
    LogicalKey::Character(InputText::new(value).unwrap())
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
    assert_eq!(
        plus,
        KeyChord::letter(
            Modifiers::NONE.with_control(),
            &InputText::new("+").unwrap()
        )
    );
    let alt_graph: KeyChord = "ctrl+altgr+l".parse().unwrap();
    let expected = KeyChord::letter(
        Modifiers::NONE.with_control().with_alt_graph(),
        &InputText::new("l").unwrap(),
    );
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
        Err(KeymapError::UnknownAction("reload_everything".to_owned()))
    );
}

#[test]
fn from_bindings_keeps_valid_bindings_and_reports_the_rest() {
    let load = Keymap::from_bindings([
        KeymapBinding::new("ctrl+k", "focus_omnibox"),
        KeymapBinding::new("ctrl+", "focus_omnibox"),
        KeymapBinding::new("ctrl+j", "teleport"),
    ]);
    let ctrl = Modifiers::NONE.with_control();
    let ctrl_k = press(character("k"), KeyCode::KEY_K, ctrl);
    assert_eq!(
        load.keymap().resolve(&ctrl_k),
        Some(BrowserCommand::FocusOmnibox)
    );
    assert_eq!(load.rejected().len(), 2);
    let ctrl_l = press(character("l"), KeyCode::KEY_L, ctrl);
    assert_eq!(
        load.keymap().resolve(&ctrl_l),
        None,
        "a valid set replaces the defaults"
    );
}

#[test]
fn an_entirely_invalid_set_keeps_the_default_keymap() {
    let load = Keymap::from_bindings([KeymapBinding::new("nope+", "nothing")]);
    assert_eq!(*load.keymap(), Keymap::default());
    assert_eq!(load.rejected().len(), 1);
    assert_eq!(Keymap::from_bindings([]).into_keymap(), Keymap::default());
}

#[test]
fn typing_reaches_only_the_omnibox_while_commands_reach_everyone() {
    let insert = UiInput::Insert(InputText::new("a").unwrap());
    let edit = UiInput::Edit(EditKey::Backspace);
    let command = UiInput::Command(BrowserCommand::FocusOmnibox);
    assert!(insert.reaches(FocusTarget::Omnibox) && edit.reaches(FocusTarget::Omnibox));
    assert!(!insert.reaches(FocusTarget::Content) && !edit.reaches(FocusTarget::Content));
    assert!(command.reaches(FocusTarget::Content));
}
