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
//! `#[non_exhaustive]` (see [`physical_key`](crate::infrastructure::physical_key)),
//! so that inner mapping keeps one documented wildcard arm.
//!
//! The mapping is a pure function of the event and the [`KeyboardState`] it is
//! given: updating that state from `ModifiersChanged`, `Focused` and key
//! presses is [`KeyboardState::observe`], called before the mapping.

use crate::domain::cursor::CursorIcon;
use crate::domain::event::{PointerButton, WindowEvent};
use crate::domain::surface::{PhysicalPosition, SurfaceSize};
use crate::domain::text::InputText;
use crate::infrastructure::keyboard::{KeyInput, KeyboardState};

/// Maps one `winit` window event to this port's vocabulary, or `None` when
/// the event has no representation in it yet.
///
/// `keyboard` must already have [`observe`](KeyboardState::observe)d `event`:
/// a key press is stamped with the modifiers it holds.
#[must_use]
pub fn map_window_event(
    event: winit::event::WindowEvent,
    keyboard: &KeyboardState,
) -> Option<WindowEvent> {
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
        winit::event::WindowEvent::KeyboardInput { event, .. } => Some(WindowEvent::Key(
            keyboard.translate(&KeyInput::from_winit(&event)),
        )),
        winit::event::WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
            InputText::new(text).map(WindowEvent::TextCommitted)
        }
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
        | winit::event::WindowEvent::Ime(
            winit::event::Ime::Enabled
            | winit::event::Ime::Preedit(..)
            | winit::event::Ime::Disabled,
        )
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

/// Maps a port [`CursorIcon`] to the `winit` cursor it names.
#[must_use]
pub const fn map_cursor_icon(icon: CursorIcon) -> winit::window::CursorIcon {
    match icon {
        CursorIcon::Default => winit::window::CursorIcon::Default,
        CursorIcon::Pointer => winit::window::CursorIcon::Pointer,
        CursorIcon::Text => winit::window::CursorIcon::Text,
        CursorIcon::Wait => winit::window::CursorIcon::Wait,
        CursorIcon::Progress => winit::window::CursorIcon::Progress,
        CursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
        CursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
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
