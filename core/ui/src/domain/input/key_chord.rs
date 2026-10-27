//! [`KeyChord`] — a modifier set plus one key, the unit a keymap binds.

use core::str::FromStr;

use window::{InputText, KeyCode, KeyEvent, LogicalKey, Modifiers};

use super::keymap_error::KeymapError;

/// The key half of a chord.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ChordKey {
    /// What the key means under the active layout. Characters are compared
    /// case-folded; Shift is part of the chord's modifiers.
    Logical(LogicalKey),
    /// The key's position, for layouts whose logical key is not Latin
    /// (`ADR-0027`).
    Physical(KeyCode),
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

    /// The chord for a printable `letter` with `modifiers`.
    #[must_use]
    pub fn letter(modifiers: Modifiers, letter: &InputText) -> Self {
        Self::new(
            modifiers,
            ChordKey::Logical(fold_case(LogicalKey::Character(letter.clone()))),
        )
    }

    /// The chords `event` can match, logical first, physical second.
    pub(super) fn candidates(event: &KeyEvent) -> [Self; 2] {
        let modifiers = event.modifiers();
        [
            Self::new(
                modifiers,
                ChordKey::Logical(fold_case(event.logical().clone())),
            ),
            Self::new(modifiers, ChordKey::Physical(event.physical())),
        ]
    }
}

fn fold_case(key: LogicalKey) -> LogicalKey {
    let LogicalKey::Character(text) = &key else {
        return key;
    };
    InputText::new(text.as_str().to_lowercase()).map_or(key, LogicalKey::Character)
}

impl FromStr for KeyChord {
    type Err = KeymapError;

    /// Parses `"ctrl+l"`, `"alt+d"`, `"ctrl+shift+arrowleft"`, `"ctrl++"`.
    /// Words are case-insensitive; the key is the last word.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (modifier_words, key_word) = split_chord(text)?;
        let modifiers = parse_modifiers(text, modifier_words)?;
        let key = parse_key(text, key_word)?;
        Ok(Self::new(modifiers, key))
    }
}

/// Splits `text` into its modifier words and key word, where the key may be
/// the literal `+`.
fn split_chord(text: &str) -> Result<(&str, &str), KeymapError> {
    let missing = || KeymapError::MissingKey(text.to_owned());
    let Some((leading, key)) = text.rsplit_once('+') else {
        return if text.is_empty() {
            Err(missing())
        } else {
            Ok(("", text))
        };
    };
    if !key.is_empty() {
        return Ok((leading, key));
    }
    match leading.strip_suffix('+') {
        Some(modifier_words) => Ok((modifier_words, "+")),
        None if leading.is_empty() => Ok(("", "+")),
        None => Err(missing()),
    }
}

fn parse_modifiers(chord: &str, words: &str) -> Result<Modifiers, KeymapError> {
    let unknown = |word: &str| KeymapError::UnknownModifier {
        chord: chord.to_owned(),
        modifier: word.to_owned(),
    };
    let mut modifiers = Modifiers::NONE;
    for word in words.split('+').filter(|word| !word.is_empty()) {
        modifiers = match word.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.with_control(),
            "shift" => modifiers.with_shift(),
            "alt" | "option" => modifiers.with_alt(),
            "altgr" | "altgraph" => modifiers.with_alt_graph(),
            "super" | "cmd" | "meta" | "win" => modifiers.with_super_key(),
            _ => return Err(unknown(word)),
        };
    }
    Ok(modifiers)
}

fn parse_key(chord: &str, word: &str) -> Result<ChordKey, KeymapError> {
    if let Some(named) = named_key(&word.to_lowercase()) {
        return Ok(ChordKey::Logical(named));
    }
    let mut characters = word.chars();
    let single = characters.next().filter(|_| characters.next().is_none());
    let character = single.and_then(|only| InputText::new(only.to_string()));
    character
        .map(|text| ChordKey::Logical(fold_case(LogicalKey::Character(text))))
        .ok_or_else(|| KeymapError::UnknownKey {
            chord: chord.to_owned(),
            key: word.to_owned(),
        })
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
