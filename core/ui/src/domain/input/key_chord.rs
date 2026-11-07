//! [`KeyChord`] — a modifier set plus one key, the unit a keymap binds.

use core::str::FromStr;

use window::{InputText, KeyCode, KeyEvent, LogicalKey, Modifiers};

use super::keymap_binding::{ChordText, ChordWord};
use super::keymap_error::KeymapError;

/// The key half of a chord.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ChordKey {
    /// What the key means under the active layout. Characters are compared
    /// case-folded; Shift is part of the chord's modifiers.
    Logical(LogicalKey),
    /// The key's position. A chord is matched against it only when the logical
    /// key is a non-ASCII character (`ADR-0027`); explicit `code:` chords are
    /// for those layouts.
    Physical(KeyCode),
}

impl ChordKey {
    /// The logical key for `key`, with characters case-folded.
    fn logical(key: LogicalKey) -> Self {
        let LogicalKey::Character(text) = &key else {
            return Self::Logical(key);
        };
        let folded =
            InputText::new(text.as_str().to_lowercase()).map_or(key, LogicalKey::Character);
        Self::Logical(folded)
    }
}

/// Modifiers and a key. Modifiers match exactly, so Ctrl+Shift+L is not Ctrl+L
/// and a chord that includes `AltGr` stays bindable.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyChord {
    modifiers: Modifiers,
    key: ChordKey,
}

impl KeyChord {
    #[must_use]
    pub const fn new(modifiers: Modifiers, key: ChordKey) -> Self {
        Self { modifiers, key }
    }

    /// The chord for `event`'s logical key.
    pub(super) fn logical_of(event: &KeyEvent) -> Self {
        Self::new(
            event.modifiers(),
            ChordKey::logical(event.logical().clone()),
        )
    }

    /// The chord for `event`'s position, or `None` when the logical key already
    /// says what was pressed. Only a logical character outside ASCII (Cyrillic,
    /// Greek, ...) falls back to its position: on Dvorak or Colemak the logical
    /// key is Latin and is the whole truth, so Ctrl+N is never Ctrl+L.
    pub(super) fn physical_of(event: &KeyEvent) -> Option<Self> {
        let LogicalKey::Character(text) = event.logical() else {
            return None;
        };
        let non_latin = !text.as_str().is_ascii();
        non_latin.then(|| Self::new(event.modifiers(), ChordKey::Physical(event.physical())))
    }

    /// Parses `text`, remembering it for the error if it is malformed.
    fn parse(text: &str) -> Result<Self, KeymapError> {
        let chord = ChordText::from(text);
        let (modifier_words, key_word) = split_chord(text);
        if key_word.is_empty() {
            return Err(KeymapError::MissingKey(chord));
        }
        let modifiers = parse_modifiers(&chord, modifier_words)?;
        let key = parse_key(key_word).ok_or_else(|| KeymapError::UnknownKey {
            chord,
            key: ChordWord::from(key_word),
        })?;
        Ok(Self::new(modifiers, key))
    }
}

impl FromStr for KeyChord {
    type Err = KeymapError;

    /// Parses `"ctrl+l"`, `"alt+d"`, `"ctrl+shift+arrowleft"`, `"ctrl++"` and
    /// the positional `"ctrl+code:l"`. Words are case-insensitive; the key is
    /// the last word.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

/// Splits `text` into its modifier words and key word; the key may be `+`.
fn split_chord(text: &str) -> (&str, &str) {
    if text == "+" {
        return ("", "+");
    }
    if let Some(modifier_words) = text.strip_suffix("++") {
        return (modifier_words, "+");
    }
    text.rsplit_once('+').unwrap_or(("", text))
}

fn parse_modifiers(chord: &ChordText, words: &str) -> Result<Modifiers, KeymapError> {
    let mut modifiers = Modifiers::NONE;
    for word in words.split('+').filter(|word| !word.is_empty()) {
        modifiers = match word.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.with_control(),
            "shift" => modifiers.with_shift(),
            "alt" | "option" => modifiers.with_alt(),
            "altgr" | "altgraph" => modifiers.with_alt_graph(),
            "super" | "cmd" | "meta" | "win" => modifiers.with_super_key(),
            _ => {
                return Err(KeymapError::UnknownModifier {
                    chord: chord.clone(),
                    modifier: ChordWord::from(word),
                });
            }
        };
    }
    Ok(modifiers)
}

fn parse_key(word: &str) -> Option<ChordKey> {
    let lowered = word.to_lowercase();
    if let Some(position) = lowered.strip_prefix("code:") {
        return physical_key(position).map(ChordKey::Physical);
    }
    if let Some(named) = named_key(&lowered) {
        return Some(ChordKey::logical(named));
    }
    let mut characters = word.chars();
    let (Some(only), None) = (characters.next(), characters.next()) else {
        return None;
    };
    InputText::new(only.to_string()).map(|text| ChordKey::logical(LogicalKey::Character(text)))
}

fn named_key(word: &str) -> Option<LogicalKey> {
    Some(match word {
        "enter" | "return" => LogicalKey::Enter,
        "tab" => LogicalKey::Tab,
        "space" => LogicalKey::Space,
        "backspace" => LogicalKey::Backspace,
        "delete" => LogicalKey::Delete,
        "escape" | "esc" => LogicalKey::Escape,
        "insert" => LogicalKey::Insert,
        "arrowup" | "up" => LogicalKey::ArrowUp,
        "arrowdown" | "down" => LogicalKey::ArrowDown,
        "arrowleft" | "left" => LogicalKey::ArrowLeft,
        "arrowright" | "right" => LogicalKey::ArrowRight,
        "home" => LogicalKey::Home,
        "end" => LogicalKey::End,
        "pageup" => LogicalKey::PageUp,
        "pagedown" => LogicalKey::PageDown,
        _ => return None,
    })
}

const LETTER_KEYS: [KeyCode; 26] = [
    KeyCode::KEY_A,
    KeyCode::KEY_B,
    KeyCode::KEY_C,
    KeyCode::KEY_D,
    KeyCode::KEY_E,
    KeyCode::KEY_F,
    KeyCode::KEY_G,
    KeyCode::KEY_H,
    KeyCode::KEY_I,
    KeyCode::KEY_J,
    KeyCode::KEY_K,
    KeyCode::KEY_L,
    KeyCode::KEY_M,
    KeyCode::KEY_N,
    KeyCode::KEY_O,
    KeyCode::KEY_P,
    KeyCode::KEY_Q,
    KeyCode::KEY_R,
    KeyCode::KEY_S,
    KeyCode::KEY_T,
    KeyCode::KEY_U,
    KeyCode::KEY_V,
    KeyCode::KEY_W,
    KeyCode::KEY_X,
    KeyCode::KEY_Y,
    KeyCode::KEY_Z,
];

const DIGIT_KEYS: [KeyCode; 10] = [
    KeyCode::DIGIT0,
    KeyCode::DIGIT1,
    KeyCode::DIGIT2,
    KeyCode::DIGIT3,
    KeyCode::DIGIT4,
    KeyCode::DIGIT5,
    KeyCode::DIGIT6,
    KeyCode::DIGIT7,
    KeyCode::DIGIT8,
    KeyCode::DIGIT9,
];

/// The key at the position of the Latin letter or digit `name` (`"l"`, `"7"`).
fn physical_key(name: &str) -> Option<KeyCode> {
    let mut characters = name.chars();
    let (Some(only), None) = (characters.next(), characters.next()) else {
        return None;
    };
    if let Some(index) = only.to_digit(10) {
        return DIGIT_KEYS.get(usize::try_from(index).ok()?).copied();
    }
    let offset = u32::from(only).checked_sub(u32::from('a'))?;
    LETTER_KEYS.get(usize::try_from(offset).ok()?).copied()
}
