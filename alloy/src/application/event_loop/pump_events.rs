//! What one `pump_events` drain observed, folded so the loop decides once per
//! cycle.

use window::{PhysicalPosition, PointerButton, PumpStatus, SurfaceSize, WindowEvent};

use super::keyboard_input::KeyboardInput;

/// Only the *last* resize counts, any number of redraw requests cost one
/// repaint, and a click lands wherever the pointer was at that moment in the
/// batch.
#[derive(Default)]
pub(super) struct PumpEvents {
    /// A window event or a finished fetch was seen this cycle.
    did_work: bool,
    close_requested: bool,
    latest_resize: Option<SurfaceSize>,
    needs_repaint: bool,
    pointer_position: Option<PhysicalPosition>,
    clicked_at: Option<PhysicalPosition>,
    /// Key presses and input-method commits, handled after the pointer so the
    /// chrome's router, not this loop, decides what each means (`ADR-0027`).
    keyboard: Vec<KeyboardInput>,
}

impl PumpEvents {
    pub(super) fn starting_at(pointer_position: Option<PhysicalPosition>) -> Self {
        Self {
            pointer_position,
            ..Self::default()
        }
    }

    pub(super) fn observe(&mut self, event: &WindowEvent) {
        self.did_work = true;
        match event {
            WindowEvent::CloseRequested => self.close_requested = true,
            WindowEvent::Resized(viewport) => self.latest_resize = Some(*viewport),
            WindowEvent::RedrawRequested => self.needs_repaint = true,
            WindowEvent::PointerMoved { position } => self.pointer_position = Some(*position),
            WindowEvent::PointerButton {
                button: PointerButton::Left,
                pressed: true,
            } => self.clicked_at = self.pointer_position.or(self.clicked_at),
            _ => {}
        }
        self.keyboard.extend(KeyboardInput::from_event(event));
    }

    pub(super) fn status(&self, window_status: PumpStatus) -> PumpStatus {
        if self.close_requested || window_status == PumpStatus::Exit {
            return PumpStatus::Exit;
        }
        PumpStatus::Continue
    }

    pub(super) const fn observe_fetch(&mut self) {
        self.did_work = true;
    }

    /// Whether the cycle saw a window event or a finished fetch.
    pub(super) const fn did_work(&self) -> bool {
        self.did_work
    }

    pub(super) const fn needs_repaint(&self) -> bool {
        self.needs_repaint
    }

    pub(super) const fn pointer_position(&self) -> Option<PhysicalPosition> {
        self.pointer_position
    }

    /// What this cycle hands to `BrowserApp`; the keyboard inputs leave with it.
    pub(super) fn take_input(&mut self) -> WindowInput {
        WindowInput {
            latest_resize: self.latest_resize,
            clicked_at: self.clicked_at,
            keyboard: std::mem::take(&mut self.keyboard),
        }
    }
}

/// The folded window input of one cycle, as `BrowserApp` consumes it: the last
/// resize, the click, and every keyboard input in arrival order.
pub struct WindowInput {
    latest_resize: Option<SurfaceSize>,
    clicked_at: Option<PhysicalPosition>,
    keyboard: Vec<KeyboardInput>,
}

impl WindowInput {
    pub const fn latest_resize(&self) -> Option<SurfaceSize> {
        self.latest_resize
    }

    pub const fn clicked_at(&self) -> Option<PhysicalPosition> {
        self.clicked_at
    }

    pub fn into_keyboard(self) -> Vec<KeyboardInput> {
        self.keyboard
    }
}
