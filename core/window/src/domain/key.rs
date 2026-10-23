//! [`KeyCode`] — a keyboard key's physical position, independent of layout.
//!
//! Split out of `event.rs` to keep that file within Object-Calisthenics'
//! entity-size guidance: the physical key set below is a closed, ~200-member
//! vocabulary (the UI Events `KeyboardEvent.code` list `winit::keyboard::KeyCode`
//! mirrors), so it reads as flat data — the same shape as
//! `network::StatusCode`'s named-constants-over-a-newtype pattern — not as a
//! behaviour-carrying entity.
//!
//! `core/window` names no `winit` type in `domain` or `application`
//! (`ADR-0011` item 2): `infrastructure::event_map` is the only place that
//! knows `winit::keyboard::KeyCode` exists. Its mapping to these constants is
//! exhaustive over every variant `winit` ships today; `winit::keyboard::KeyCode`
//! is itself `#[non_exhaustive]`, so the mapper still needs one wildcard arm
//! for a future addition — documented there, not silently absorbed here.

use core::fmt;

/// A physical keyboard key, identified by position rather than by the
/// character it produces under the active layout.
///
/// Mirrors `winit::keyboard::KeyCode`'s own doc: this "mostly conforms to the
/// UI Events Specification's `KeyboardEvent.code`".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyCode(u32);

impl KeyCode {
    /// A key `infrastructure::event_map` could not resolve to a known
    /// physical position — `winit::keyboard::PhysicalKey::Unidentified`, or a
    /// future `winit::keyboard::KeyCode` variant this port has not named yet.
    pub const UNIDENTIFIED: Self = Self(0);

    pub const BACKQUOTE: Self = Self(1);
    pub const BACKSLASH: Self = Self(2);
    pub const BRACKET_LEFT: Self = Self(3);
    pub const BRACKET_RIGHT: Self = Self(4);
    pub const COMMA: Self = Self(5);
    pub const DIGIT0: Self = Self(6);
    pub const DIGIT1: Self = Self(7);
    pub const DIGIT2: Self = Self(8);
    pub const DIGIT3: Self = Self(9);
    pub const DIGIT4: Self = Self(10);
    pub const DIGIT5: Self = Self(11);
    pub const DIGIT6: Self = Self(12);
    pub const DIGIT7: Self = Self(13);
    pub const DIGIT8: Self = Self(14);
    pub const DIGIT9: Self = Self(15);
    pub const EQUAL: Self = Self(16);
    pub const INTL_BACKSLASH: Self = Self(17);
    pub const INTL_RO: Self = Self(18);
    pub const INTL_YEN: Self = Self(19);
    pub const KEY_A: Self = Self(20);
    pub const KEY_B: Self = Self(21);
    pub const KEY_C: Self = Self(22);
    pub const KEY_D: Self = Self(23);
    pub const KEY_E: Self = Self(24);
    pub const KEY_F: Self = Self(25);
    pub const KEY_G: Self = Self(26);
    pub const KEY_H: Self = Self(27);
    pub const KEY_I: Self = Self(28);
    pub const KEY_J: Self = Self(29);
    pub const KEY_K: Self = Self(30);
    pub const KEY_L: Self = Self(31);
    pub const KEY_M: Self = Self(32);
    pub const KEY_N: Self = Self(33);
    pub const KEY_O: Self = Self(34);
    pub const KEY_P: Self = Self(35);
    pub const KEY_Q: Self = Self(36);
    pub const KEY_R: Self = Self(37);
    pub const KEY_S: Self = Self(38);
    pub const KEY_T: Self = Self(39);
    pub const KEY_U: Self = Self(40);
    pub const KEY_V: Self = Self(41);
    pub const KEY_W: Self = Self(42);
    pub const KEY_X: Self = Self(43);
    pub const KEY_Y: Self = Self(44);
    pub const KEY_Z: Self = Self(45);
    pub const MINUS: Self = Self(46);
    pub const PERIOD: Self = Self(47);
    pub const QUOTE: Self = Self(48);
    pub const SEMICOLON: Self = Self(49);
    pub const SLASH: Self = Self(50);
    pub const ALT_LEFT: Self = Self(51);
    pub const ALT_RIGHT: Self = Self(52);
    pub const BACKSPACE: Self = Self(53);
    pub const CAPS_LOCK: Self = Self(54);
    pub const CONTEXT_MENU: Self = Self(55);
    pub const CONTROL_LEFT: Self = Self(56);
    pub const CONTROL_RIGHT: Self = Self(57);
    pub const ENTER: Self = Self(58);
    pub const SUPER_LEFT: Self = Self(59);
    pub const SUPER_RIGHT: Self = Self(60);
    pub const SHIFT_LEFT: Self = Self(61);
    pub const SHIFT_RIGHT: Self = Self(62);
    pub const SPACE: Self = Self(63);
    pub const TAB: Self = Self(64);
    pub const CONVERT: Self = Self(65);
    pub const KANA_MODE: Self = Self(66);
    pub const LANG1: Self = Self(67);
    pub const LANG2: Self = Self(68);
    pub const LANG3: Self = Self(69);
    pub const LANG4: Self = Self(70);
    pub const LANG5: Self = Self(71);
    pub const NON_CONVERT: Self = Self(72);
    pub const DELETE: Self = Self(73);
    pub const END: Self = Self(74);
    pub const HELP: Self = Self(75);
    pub const HOME: Self = Self(76);
    pub const INSERT: Self = Self(77);
    pub const PAGE_DOWN: Self = Self(78);
    pub const PAGE_UP: Self = Self(79);
    pub const ARROW_DOWN: Self = Self(80);
    pub const ARROW_LEFT: Self = Self(81);
    pub const ARROW_RIGHT: Self = Self(82);
    pub const ARROW_UP: Self = Self(83);
    pub const NUM_LOCK: Self = Self(84);
    pub const NUMPAD0: Self = Self(85);
    pub const NUMPAD1: Self = Self(86);
    pub const NUMPAD2: Self = Self(87);
    pub const NUMPAD3: Self = Self(88);
    pub const NUMPAD4: Self = Self(89);
    pub const NUMPAD5: Self = Self(90);
    pub const NUMPAD6: Self = Self(91);
    pub const NUMPAD7: Self = Self(92);
    pub const NUMPAD8: Self = Self(93);
    pub const NUMPAD9: Self = Self(94);
    pub const NUMPAD_ADD: Self = Self(95);
    pub const NUMPAD_BACKSPACE: Self = Self(96);
    pub const NUMPAD_CLEAR: Self = Self(97);
    pub const NUMPAD_CLEAR_ENTRY: Self = Self(98);
    pub const NUMPAD_COMMA: Self = Self(99);
    pub const NUMPAD_DECIMAL: Self = Self(100);
    pub const NUMPAD_DIVIDE: Self = Self(101);
    pub const NUMPAD_ENTER: Self = Self(102);
    pub const NUMPAD_EQUAL: Self = Self(103);
    pub const NUMPAD_HASH: Self = Self(104);
    pub const NUMPAD_MEMORY_ADD: Self = Self(105);
    pub const NUMPAD_MEMORY_CLEAR: Self = Self(106);
    pub const NUMPAD_MEMORY_RECALL: Self = Self(107);
    pub const NUMPAD_MEMORY_STORE: Self = Self(108);
    pub const NUMPAD_MEMORY_SUBTRACT: Self = Self(109);
    pub const NUMPAD_MULTIPLY: Self = Self(110);
    pub const NUMPAD_PAREN_LEFT: Self = Self(111);
    pub const NUMPAD_PAREN_RIGHT: Self = Self(112);
    pub const NUMPAD_STAR: Self = Self(113);
    pub const NUMPAD_SUBTRACT: Self = Self(114);
    pub const ESCAPE: Self = Self(115);
    pub const FN: Self = Self(116);
    pub const FN_LOCK: Self = Self(117);
    pub const PRINT_SCREEN: Self = Self(118);
    pub const SCROLL_LOCK: Self = Self(119);
    pub const PAUSE: Self = Self(120);
    pub const BROWSER_BACK: Self = Self(121);
    pub const BROWSER_FAVORITES: Self = Self(122);
    pub const BROWSER_FORWARD: Self = Self(123);
    pub const BROWSER_HOME: Self = Self(124);
    pub const BROWSER_REFRESH: Self = Self(125);
    pub const BROWSER_SEARCH: Self = Self(126);
    pub const BROWSER_STOP: Self = Self(127);
    pub const EJECT: Self = Self(128);
    pub const LAUNCH_APP1: Self = Self(129);
    pub const LAUNCH_APP2: Self = Self(130);
    pub const LAUNCH_MAIL: Self = Self(131);
    pub const MEDIA_PLAY_PAUSE: Self = Self(132);
    pub const MEDIA_SELECT: Self = Self(133);
    pub const MEDIA_STOP: Self = Self(134);
    pub const MEDIA_TRACK_NEXT: Self = Self(135);
    pub const MEDIA_TRACK_PREVIOUS: Self = Self(136);
    pub const POWER: Self = Self(137);
    pub const SLEEP: Self = Self(138);
    pub const AUDIO_VOLUME_DOWN: Self = Self(139);
    pub const AUDIO_VOLUME_MUTE: Self = Self(140);
    pub const AUDIO_VOLUME_UP: Self = Self(141);
    pub const WAKE_UP: Self = Self(142);
    pub const META: Self = Self(143);
    pub const HYPER: Self = Self(144);
    pub const TURBO: Self = Self(145);
    pub const ABORT: Self = Self(146);
    pub const RESUME: Self = Self(147);
    pub const SUSPEND: Self = Self(148);
    pub const AGAIN: Self = Self(149);
    pub const COPY: Self = Self(150);
    pub const CUT: Self = Self(151);
    pub const FIND: Self = Self(152);
    pub const OPEN: Self = Self(153);
    pub const PASTE: Self = Self(154);
    pub const PROPS: Self = Self(155);
    pub const SELECT: Self = Self(156);
    pub const UNDO: Self = Self(157);
    pub const HIRAGANA: Self = Self(158);
    pub const KATAKANA: Self = Self(159);
    pub const F1: Self = Self(160);
    pub const F2: Self = Self(161);
    pub const F3: Self = Self(162);
    pub const F4: Self = Self(163);
    pub const F5: Self = Self(164);
    pub const F6: Self = Self(165);
    pub const F7: Self = Self(166);
    pub const F8: Self = Self(167);
    pub const F9: Self = Self(168);
    pub const F10: Self = Self(169);
    pub const F11: Self = Self(170);
    pub const F12: Self = Self(171);
    pub const F13: Self = Self(172);
    pub const F14: Self = Self(173);
    pub const F15: Self = Self(174);
    pub const F16: Self = Self(175);
    pub const F17: Self = Self(176);
    pub const F18: Self = Self(177);
    pub const F19: Self = Self(178);
    pub const F20: Self = Self(179);
    pub const F21: Self = Self(180);
    pub const F22: Self = Self(181);
    pub const F23: Self = Self(182);
    pub const F24: Self = Self(183);
    pub const F25: Self = Self(184);
    pub const F26: Self = Self(185);
    pub const F27: Self = Self(186);
    pub const F28: Self = Self(187);
    pub const F29: Self = Self(188);
    pub const F30: Self = Self(189);
    pub const F31: Self = Self(190);
    pub const F32: Self = Self(191);
    pub const F33: Self = Self(192);
    pub const F34: Self = Self(193);
    pub const F35: Self = Self(194);

    /// Wraps a raw identifier not covered by a named constant above. No
    /// current `event_map` mapping needs this — every `winit::keyboard::KeyCode`
    /// variant this port ships against has a named constant — but it keeps
    /// the type open for a future backend reporting a position this port has
    /// not named yet.
    #[must_use]
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl fmt::Display for KeyCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "key#{}", self.0)
    }
}
