//! [`WindowEvent`] — everything a
//! [`WindowSystem`](crate::application::ports::WindowSystem) can hand back
//! through `pump_events`.
//!
//! `#[non_exhaustive]`: a caller matches the variants it cares about and lets
//! a `_` arm absorb anything a future release adds. The totality obligation —
//! "no backend event is silently dropped" — sits on
//! `infrastructure::event_map`'s mapping *function*, which is written as an
//! exhaustive match with no wildcard over `winit::event::WindowEvent` itself
//! (`ADR-0011` item 3).

use crate::domain::key_event::KeyEvent;
use crate::domain::surface::{PhysicalPosition, SurfaceSize};
use crate::domain::text::InputText;

/// One event a window's backend observed.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    /// The surface was resized — once at window creation, and again on every
    /// user resize.
    Resized(SurfaceSize),
    /// The user asked the window to close (the OS close button, `Alt+F4`, …).
    /// The port never closes a window on the caller's behalf.
    CloseRequested,
    /// The pointer moved within the window, in physical pixels.
    PointerMoved { position: PhysicalPosition },
    /// A pointer button changed state.
    PointerButton {
        button: PointerButton,
        pressed: bool,
    },
    /// One physical key press (or auto-repeat, or release), carrying its
    /// meaning, position, text and modifiers. The adapter never classifies it
    /// as shortcut or typing — the consumer's keymap does (`ADR-0027`).
    Key(KeyEvent),
    /// Text an input method committed, with no single key behind it
    /// (composition, emoji pickers, dictation). Text a key press produces
    /// travels in [`KeyEvent::text`] instead.
    TextCommitted(InputText),
    /// A scroll wheel or touchpad-pan gesture. No invariant to protect (any
    /// finite delta is a legal reading), so `delta_x`/`delta_y` stay bare
    /// `f64` rather than a dedicated newtype.
    Scroll { delta_x: f64, delta_y: f64 },
    /// The surface should be repainted and presented.
    RedrawRequested,
}

/// Which pointer button changed state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PointerButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    /// A button this port has not named, carrying the backend's own index.
    Other(u16),
}
