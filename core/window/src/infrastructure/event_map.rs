//! The explicit, exhaustive `winit::event::WindowEvent` → `domain::WindowEvent`
//! mapping (`ADR-0011` item 2).
//!
//! No `winit` type crosses into `domain` or `application` — this module,
//! `winit_system` and `softbuffer_presenter` are the only places in this
//! crate that name one.
//!
//! [`map_window_event`] has no wildcard arm on purpose: `winit::event::WindowEvent`
//! is not `#[non_exhaustive]`, so a `winit` upgrade that adds a variant fails
//! to compile here instead of silently dropping the new event — the
//! "totality" pin of `PRD-010`. Variants this port does not (yet) represent
//! in its own vocabulary are named explicitly and mapped to `None`, which is
//! a reviewable decision, not a silent one. `winit::keyboard::KeyCode` *is*
//! `#[non_exhaustive]` (see [`map_key_code`]), so that inner mapping keeps one
//! documented wildcard arm.

use winit::keyboard::PhysicalKey;

use crate::domain::event::{PointerButton, WindowEvent};
use crate::domain::key::KeyCode;
use crate::domain::surface::{PhysicalPosition, SurfaceSize};

/// Maps one `winit` window event to this port's vocabulary, or `None` when
/// the event has no representation in it yet.
#[must_use]
pub fn map_window_event(event: winit::event::WindowEvent) -> Option<WindowEvent> {
    match event {
        winit::event::WindowEvent::Resized(size) => {
            SurfaceSize::new(size.width, size.height).map(WindowEvent::Resized)
        }
        winit::event::WindowEvent::CloseRequested => Some(WindowEvent::CloseRequested),
        winit::event::WindowEvent::CursorMoved { position, .. } => {
            Some(WindowEvent::PointerMoved {
                position: PhysicalPosition::new(position.x, position.y),
            })
        }
        winit::event::WindowEvent::MouseInput { state, button, .. } => {
            Some(WindowEvent::PointerButton {
                button: map_mouse_button(button),
                pressed: state.is_pressed(),
            })
        }
        winit::event::WindowEvent::KeyboardInput { event, .. } => Some(WindowEvent::Key {
            code: map_physical_key(event.physical_key),
            pressed: event.state.is_pressed(),
        }),
        winit::event::WindowEvent::MouseWheel { delta, .. } => Some(map_scroll_delta(delta)),
        winit::event::WindowEvent::RedrawRequested => Some(WindowEvent::RedrawRequested),

        // Not represented in this port's vocabulary yet (PRD-010): observed,
        // reviewed, and explicitly dropped here rather than silently ignored
        // by a wildcard arm.
        winit::event::WindowEvent::ActivationTokenDone { .. }
        | winit::event::WindowEvent::Moved(_)
        | winit::event::WindowEvent::Destroyed
        | winit::event::WindowEvent::DroppedFile(_)
        | winit::event::WindowEvent::HoveredFile(_)
        | winit::event::WindowEvent::HoveredFileCancelled
        | winit::event::WindowEvent::Focused(_)
        | winit::event::WindowEvent::ModifiersChanged(_)
        | winit::event::WindowEvent::Ime(_)
        | winit::event::WindowEvent::CursorEntered { .. }
        | winit::event::WindowEvent::CursorLeft { .. }
        | winit::event::WindowEvent::PinchGesture { .. }
        | winit::event::WindowEvent::PanGesture { .. }
        | winit::event::WindowEvent::DoubleTapGesture { .. }
        | winit::event::WindowEvent::RotationGesture { .. }
        | winit::event::WindowEvent::TouchpadPressure { .. }
        | winit::event::WindowEvent::AxisMotion { .. }
        | winit::event::WindowEvent::Touch(_)
        | winit::event::WindowEvent::ScaleFactorChanged { .. }
        | winit::event::WindowEvent::ThemeChanged(_)
        | winit::event::WindowEvent::Occluded(_) => None,
    }
}

fn map_scroll_delta(delta: winit::event::MouseScrollDelta) -> WindowEvent {
    match delta {
        winit::event::MouseScrollDelta::LineDelta(x, y) => WindowEvent::Scroll {
            delta_x: f64::from(x),
            delta_y: f64::from(y),
        },
        winit::event::MouseScrollDelta::PixelDelta(position) => WindowEvent::Scroll {
            delta_x: position.x,
            delta_y: position.y,
        },
    }
}

const fn map_mouse_button(button: winit::event::MouseButton) -> PointerButton {
    match button {
        winit::event::MouseButton::Left => PointerButton::Left,
        winit::event::MouseButton::Right => PointerButton::Right,
        winit::event::MouseButton::Middle => PointerButton::Middle,
        winit::event::MouseButton::Back => PointerButton::Back,
        winit::event::MouseButton::Forward => PointerButton::Forward,
        winit::event::MouseButton::Other(code) => PointerButton::Other(code),
    }
}

const fn map_physical_key(key: PhysicalKey) -> KeyCode {
    match key {
        PhysicalKey::Code(code) => map_key_code(code),
        // No native scancode is carried across (`NativeKeyCode` is itself
        // per-platform); the physical position is simply not one `winit`
        // could name.
        PhysicalKey::Unidentified(_) => KeyCode::UNIDENTIFIED,
    }
}

/// `winit::keyboard::KeyCode` is `#[non_exhaustive]`, so this keeps exactly
/// one wildcard arm — every variant it ships against today is still named
/// explicitly above it, and a future addition degrades to
/// [`KeyCode::UNIDENTIFIED`] rather than failing to compile.
///
/// A flat, mechanical, ~200-arm match over a closed physical-key vocabulary —
/// `too_many_lines` measures line count, not complexity, and splitting this
/// across helper functions would only add indirection between a `winit`
/// variant and its named constant.
#[allow(clippy::too_many_lines)]
const fn map_key_code(code: winit::keyboard::KeyCode) -> KeyCode {
    match code {
        winit::keyboard::KeyCode::Backquote => KeyCode::BACKQUOTE,
        winit::keyboard::KeyCode::Backslash => KeyCode::BACKSLASH,
        winit::keyboard::KeyCode::BracketLeft => KeyCode::BRACKET_LEFT,
        winit::keyboard::KeyCode::BracketRight => KeyCode::BRACKET_RIGHT,
        winit::keyboard::KeyCode::Comma => KeyCode::COMMA,
        winit::keyboard::KeyCode::Digit0 => KeyCode::DIGIT0,
        winit::keyboard::KeyCode::Digit1 => KeyCode::DIGIT1,
        winit::keyboard::KeyCode::Digit2 => KeyCode::DIGIT2,
        winit::keyboard::KeyCode::Digit3 => KeyCode::DIGIT3,
        winit::keyboard::KeyCode::Digit4 => KeyCode::DIGIT4,
        winit::keyboard::KeyCode::Digit5 => KeyCode::DIGIT5,
        winit::keyboard::KeyCode::Digit6 => KeyCode::DIGIT6,
        winit::keyboard::KeyCode::Digit7 => KeyCode::DIGIT7,
        winit::keyboard::KeyCode::Digit8 => KeyCode::DIGIT8,
        winit::keyboard::KeyCode::Digit9 => KeyCode::DIGIT9,
        winit::keyboard::KeyCode::Equal => KeyCode::EQUAL,
        winit::keyboard::KeyCode::IntlBackslash => KeyCode::INTL_BACKSLASH,
        winit::keyboard::KeyCode::IntlRo => KeyCode::INTL_RO,
        winit::keyboard::KeyCode::IntlYen => KeyCode::INTL_YEN,
        winit::keyboard::KeyCode::KeyA => KeyCode::KEY_A,
        winit::keyboard::KeyCode::KeyB => KeyCode::KEY_B,
        winit::keyboard::KeyCode::KeyC => KeyCode::KEY_C,
        winit::keyboard::KeyCode::KeyD => KeyCode::KEY_D,
        winit::keyboard::KeyCode::KeyE => KeyCode::KEY_E,
        winit::keyboard::KeyCode::KeyF => KeyCode::KEY_F,
        winit::keyboard::KeyCode::KeyG => KeyCode::KEY_G,
        winit::keyboard::KeyCode::KeyH => KeyCode::KEY_H,
        winit::keyboard::KeyCode::KeyI => KeyCode::KEY_I,
        winit::keyboard::KeyCode::KeyJ => KeyCode::KEY_J,
        winit::keyboard::KeyCode::KeyK => KeyCode::KEY_K,
        winit::keyboard::KeyCode::KeyL => KeyCode::KEY_L,
        winit::keyboard::KeyCode::KeyM => KeyCode::KEY_M,
        winit::keyboard::KeyCode::KeyN => KeyCode::KEY_N,
        winit::keyboard::KeyCode::KeyO => KeyCode::KEY_O,
        winit::keyboard::KeyCode::KeyP => KeyCode::KEY_P,
        winit::keyboard::KeyCode::KeyQ => KeyCode::KEY_Q,
        winit::keyboard::KeyCode::KeyR => KeyCode::KEY_R,
        winit::keyboard::KeyCode::KeyS => KeyCode::KEY_S,
        winit::keyboard::KeyCode::KeyT => KeyCode::KEY_T,
        winit::keyboard::KeyCode::KeyU => KeyCode::KEY_U,
        winit::keyboard::KeyCode::KeyV => KeyCode::KEY_V,
        winit::keyboard::KeyCode::KeyW => KeyCode::KEY_W,
        winit::keyboard::KeyCode::KeyX => KeyCode::KEY_X,
        winit::keyboard::KeyCode::KeyY => KeyCode::KEY_Y,
        winit::keyboard::KeyCode::KeyZ => KeyCode::KEY_Z,
        winit::keyboard::KeyCode::Minus => KeyCode::MINUS,
        winit::keyboard::KeyCode::Period => KeyCode::PERIOD,
        winit::keyboard::KeyCode::Quote => KeyCode::QUOTE,
        winit::keyboard::KeyCode::Semicolon => KeyCode::SEMICOLON,
        winit::keyboard::KeyCode::Slash => KeyCode::SLASH,
        winit::keyboard::KeyCode::AltLeft => KeyCode::ALT_LEFT,
        winit::keyboard::KeyCode::AltRight => KeyCode::ALT_RIGHT,
        winit::keyboard::KeyCode::Backspace => KeyCode::BACKSPACE,
        winit::keyboard::KeyCode::CapsLock => KeyCode::CAPS_LOCK,
        winit::keyboard::KeyCode::ContextMenu => KeyCode::CONTEXT_MENU,
        winit::keyboard::KeyCode::ControlLeft => KeyCode::CONTROL_LEFT,
        winit::keyboard::KeyCode::ControlRight => KeyCode::CONTROL_RIGHT,
        winit::keyboard::KeyCode::Enter => KeyCode::ENTER,
        winit::keyboard::KeyCode::SuperLeft => KeyCode::SUPER_LEFT,
        winit::keyboard::KeyCode::SuperRight => KeyCode::SUPER_RIGHT,
        winit::keyboard::KeyCode::ShiftLeft => KeyCode::SHIFT_LEFT,
        winit::keyboard::KeyCode::ShiftRight => KeyCode::SHIFT_RIGHT,
        winit::keyboard::KeyCode::Space => KeyCode::SPACE,
        winit::keyboard::KeyCode::Tab => KeyCode::TAB,
        winit::keyboard::KeyCode::Convert => KeyCode::CONVERT,
        winit::keyboard::KeyCode::KanaMode => KeyCode::KANA_MODE,
        winit::keyboard::KeyCode::Lang1 => KeyCode::LANG1,
        winit::keyboard::KeyCode::Lang2 => KeyCode::LANG2,
        winit::keyboard::KeyCode::Lang3 => KeyCode::LANG3,
        winit::keyboard::KeyCode::Lang4 => KeyCode::LANG4,
        winit::keyboard::KeyCode::Lang5 => KeyCode::LANG5,
        winit::keyboard::KeyCode::NonConvert => KeyCode::NON_CONVERT,
        winit::keyboard::KeyCode::Delete => KeyCode::DELETE,
        winit::keyboard::KeyCode::End => KeyCode::END,
        winit::keyboard::KeyCode::Help => KeyCode::HELP,
        winit::keyboard::KeyCode::Home => KeyCode::HOME,
        winit::keyboard::KeyCode::Insert => KeyCode::INSERT,
        winit::keyboard::KeyCode::PageDown => KeyCode::PAGE_DOWN,
        winit::keyboard::KeyCode::PageUp => KeyCode::PAGE_UP,
        winit::keyboard::KeyCode::ArrowDown => KeyCode::ARROW_DOWN,
        winit::keyboard::KeyCode::ArrowLeft => KeyCode::ARROW_LEFT,
        winit::keyboard::KeyCode::ArrowRight => KeyCode::ARROW_RIGHT,
        winit::keyboard::KeyCode::ArrowUp => KeyCode::ARROW_UP,
        winit::keyboard::KeyCode::NumLock => KeyCode::NUM_LOCK,
        winit::keyboard::KeyCode::Numpad0 => KeyCode::NUMPAD0,
        winit::keyboard::KeyCode::Numpad1 => KeyCode::NUMPAD1,
        winit::keyboard::KeyCode::Numpad2 => KeyCode::NUMPAD2,
        winit::keyboard::KeyCode::Numpad3 => KeyCode::NUMPAD3,
        winit::keyboard::KeyCode::Numpad4 => KeyCode::NUMPAD4,
        winit::keyboard::KeyCode::Numpad5 => KeyCode::NUMPAD5,
        winit::keyboard::KeyCode::Numpad6 => KeyCode::NUMPAD6,
        winit::keyboard::KeyCode::Numpad7 => KeyCode::NUMPAD7,
        winit::keyboard::KeyCode::Numpad8 => KeyCode::NUMPAD8,
        winit::keyboard::KeyCode::Numpad9 => KeyCode::NUMPAD9,
        winit::keyboard::KeyCode::NumpadAdd => KeyCode::NUMPAD_ADD,
        winit::keyboard::KeyCode::NumpadBackspace => KeyCode::NUMPAD_BACKSPACE,
        winit::keyboard::KeyCode::NumpadClear => KeyCode::NUMPAD_CLEAR,
        winit::keyboard::KeyCode::NumpadClearEntry => KeyCode::NUMPAD_CLEAR_ENTRY,
        winit::keyboard::KeyCode::NumpadComma => KeyCode::NUMPAD_COMMA,
        winit::keyboard::KeyCode::NumpadDecimal => KeyCode::NUMPAD_DECIMAL,
        winit::keyboard::KeyCode::NumpadDivide => KeyCode::NUMPAD_DIVIDE,
        winit::keyboard::KeyCode::NumpadEnter => KeyCode::NUMPAD_ENTER,
        winit::keyboard::KeyCode::NumpadEqual => KeyCode::NUMPAD_EQUAL,
        winit::keyboard::KeyCode::NumpadHash => KeyCode::NUMPAD_HASH,
        winit::keyboard::KeyCode::NumpadMemoryAdd => KeyCode::NUMPAD_MEMORY_ADD,
        winit::keyboard::KeyCode::NumpadMemoryClear => KeyCode::NUMPAD_MEMORY_CLEAR,
        winit::keyboard::KeyCode::NumpadMemoryRecall => KeyCode::NUMPAD_MEMORY_RECALL,
        winit::keyboard::KeyCode::NumpadMemoryStore => KeyCode::NUMPAD_MEMORY_STORE,
        winit::keyboard::KeyCode::NumpadMemorySubtract => KeyCode::NUMPAD_MEMORY_SUBTRACT,
        winit::keyboard::KeyCode::NumpadMultiply => KeyCode::NUMPAD_MULTIPLY,
        winit::keyboard::KeyCode::NumpadParenLeft => KeyCode::NUMPAD_PAREN_LEFT,
        winit::keyboard::KeyCode::NumpadParenRight => KeyCode::NUMPAD_PAREN_RIGHT,
        winit::keyboard::KeyCode::NumpadStar => KeyCode::NUMPAD_STAR,
        winit::keyboard::KeyCode::NumpadSubtract => KeyCode::NUMPAD_SUBTRACT,
        winit::keyboard::KeyCode::Escape => KeyCode::ESCAPE,
        winit::keyboard::KeyCode::Fn => KeyCode::FN,
        winit::keyboard::KeyCode::FnLock => KeyCode::FN_LOCK,
        winit::keyboard::KeyCode::PrintScreen => KeyCode::PRINT_SCREEN,
        winit::keyboard::KeyCode::ScrollLock => KeyCode::SCROLL_LOCK,
        winit::keyboard::KeyCode::Pause => KeyCode::PAUSE,
        winit::keyboard::KeyCode::BrowserBack => KeyCode::BROWSER_BACK,
        winit::keyboard::KeyCode::BrowserFavorites => KeyCode::BROWSER_FAVORITES,
        winit::keyboard::KeyCode::BrowserForward => KeyCode::BROWSER_FORWARD,
        winit::keyboard::KeyCode::BrowserHome => KeyCode::BROWSER_HOME,
        winit::keyboard::KeyCode::BrowserRefresh => KeyCode::BROWSER_REFRESH,
        winit::keyboard::KeyCode::BrowserSearch => KeyCode::BROWSER_SEARCH,
        winit::keyboard::KeyCode::BrowserStop => KeyCode::BROWSER_STOP,
        winit::keyboard::KeyCode::Eject => KeyCode::EJECT,
        winit::keyboard::KeyCode::LaunchApp1 => KeyCode::LAUNCH_APP1,
        winit::keyboard::KeyCode::LaunchApp2 => KeyCode::LAUNCH_APP2,
        winit::keyboard::KeyCode::LaunchMail => KeyCode::LAUNCH_MAIL,
        winit::keyboard::KeyCode::MediaPlayPause => KeyCode::MEDIA_PLAY_PAUSE,
        winit::keyboard::KeyCode::MediaSelect => KeyCode::MEDIA_SELECT,
        winit::keyboard::KeyCode::MediaStop => KeyCode::MEDIA_STOP,
        winit::keyboard::KeyCode::MediaTrackNext => KeyCode::MEDIA_TRACK_NEXT,
        winit::keyboard::KeyCode::MediaTrackPrevious => KeyCode::MEDIA_TRACK_PREVIOUS,
        winit::keyboard::KeyCode::Power => KeyCode::POWER,
        winit::keyboard::KeyCode::Sleep => KeyCode::SLEEP,
        winit::keyboard::KeyCode::AudioVolumeDown => KeyCode::AUDIO_VOLUME_DOWN,
        winit::keyboard::KeyCode::AudioVolumeMute => KeyCode::AUDIO_VOLUME_MUTE,
        winit::keyboard::KeyCode::AudioVolumeUp => KeyCode::AUDIO_VOLUME_UP,
        winit::keyboard::KeyCode::WakeUp => KeyCode::WAKE_UP,
        winit::keyboard::KeyCode::Meta => KeyCode::META,
        winit::keyboard::KeyCode::Hyper => KeyCode::HYPER,
        winit::keyboard::KeyCode::Turbo => KeyCode::TURBO,
        winit::keyboard::KeyCode::Abort => KeyCode::ABORT,
        winit::keyboard::KeyCode::Resume => KeyCode::RESUME,
        winit::keyboard::KeyCode::Suspend => KeyCode::SUSPEND,
        winit::keyboard::KeyCode::Again => KeyCode::AGAIN,
        winit::keyboard::KeyCode::Copy => KeyCode::COPY,
        winit::keyboard::KeyCode::Cut => KeyCode::CUT,
        winit::keyboard::KeyCode::Find => KeyCode::FIND,
        winit::keyboard::KeyCode::Open => KeyCode::OPEN,
        winit::keyboard::KeyCode::Paste => KeyCode::PASTE,
        winit::keyboard::KeyCode::Props => KeyCode::PROPS,
        winit::keyboard::KeyCode::Select => KeyCode::SELECT,
        winit::keyboard::KeyCode::Undo => KeyCode::UNDO,
        winit::keyboard::KeyCode::Hiragana => KeyCode::HIRAGANA,
        winit::keyboard::KeyCode::Katakana => KeyCode::KATAKANA,
        winit::keyboard::KeyCode::F1 => KeyCode::F1,
        winit::keyboard::KeyCode::F2 => KeyCode::F2,
        winit::keyboard::KeyCode::F3 => KeyCode::F3,
        winit::keyboard::KeyCode::F4 => KeyCode::F4,
        winit::keyboard::KeyCode::F5 => KeyCode::F5,
        winit::keyboard::KeyCode::F6 => KeyCode::F6,
        winit::keyboard::KeyCode::F7 => KeyCode::F7,
        winit::keyboard::KeyCode::F8 => KeyCode::F8,
        winit::keyboard::KeyCode::F9 => KeyCode::F9,
        winit::keyboard::KeyCode::F10 => KeyCode::F10,
        winit::keyboard::KeyCode::F11 => KeyCode::F11,
        winit::keyboard::KeyCode::F12 => KeyCode::F12,
        winit::keyboard::KeyCode::F13 => KeyCode::F13,
        winit::keyboard::KeyCode::F14 => KeyCode::F14,
        winit::keyboard::KeyCode::F15 => KeyCode::F15,
        winit::keyboard::KeyCode::F16 => KeyCode::F16,
        winit::keyboard::KeyCode::F17 => KeyCode::F17,
        winit::keyboard::KeyCode::F18 => KeyCode::F18,
        winit::keyboard::KeyCode::F19 => KeyCode::F19,
        winit::keyboard::KeyCode::F20 => KeyCode::F20,
        winit::keyboard::KeyCode::F21 => KeyCode::F21,
        winit::keyboard::KeyCode::F22 => KeyCode::F22,
        winit::keyboard::KeyCode::F23 => KeyCode::F23,
        winit::keyboard::KeyCode::F24 => KeyCode::F24,
        winit::keyboard::KeyCode::F25 => KeyCode::F25,
        winit::keyboard::KeyCode::F26 => KeyCode::F26,
        winit::keyboard::KeyCode::F27 => KeyCode::F27,
        winit::keyboard::KeyCode::F28 => KeyCode::F28,
        winit::keyboard::KeyCode::F29 => KeyCode::F29,
        winit::keyboard::KeyCode::F30 => KeyCode::F30,
        winit::keyboard::KeyCode::F31 => KeyCode::F31,
        winit::keyboard::KeyCode::F32 => KeyCode::F32,
        winit::keyboard::KeyCode::F33 => KeyCode::F33,
        winit::keyboard::KeyCode::F34 => KeyCode::F34,
        winit::keyboard::KeyCode::F35 => KeyCode::F35,
        _ => KeyCode::UNIDENTIFIED,
    }
}
