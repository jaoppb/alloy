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

    pub(super) const fn latest_resize(&self) -> Option<SurfaceSize> {
        self.latest_resize
    }

    pub(super) const fn clicked_at(&self) -> Option<PhysicalPosition> {
        self.clicked_at
    }

    pub(super) fn take_keyboard(&mut self) -> Vec<KeyboardInput> {
        std::mem::take(&mut self.keyboard)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use window::{PhysicalPosition, PointerButton, PumpStatus, SurfaceSize, WindowEvent};

    use super::PumpEvents;

    fn size(width: u32) -> SurfaceSize {
        SurfaceSize::new(width, 600).unwrap()
    }

    fn click() -> WindowEvent {
        WindowEvent::PointerButton {
            button: PointerButton::Left,
            pressed: true,
        }
    }

    #[test]
    fn only_the_last_resize_of_a_batch_counts() {
        let mut events = PumpEvents::default();

        events.observe(&WindowEvent::Resized(size(1024)));
        events.observe(&WindowEvent::Resized(size(640)));

        assert_eq!(events.latest_resize(), Some(size(640)));
    }

    #[test]
    fn a_click_lands_where_the_pointer_was_when_it_came() {
        let mut events = PumpEvents::default();
        let first = PhysicalPosition::new(1.0, 2.0);

        events.observe(&WindowEvent::PointerMoved { position: first });
        events.observe(&click());
        events.observe(&WindowEvent::PointerMoved {
            position: PhysicalPosition::new(9.0, 9.0),
        });

        assert_eq!(events.clicked_at(), Some(first));
        assert_eq!(
            events.pointer_position(),
            Some(PhysicalPosition::new(9.0, 9.0))
        );
    }

    #[test]
    fn many_redraw_requests_are_one_repaint_and_a_close_exits() {
        let mut events = PumpEvents::default();
        events.observe(&WindowEvent::RedrawRequested);
        events.observe(&WindowEvent::RedrawRequested);
        assert!(events.needs_repaint());
        assert_eq!(events.status(PumpStatus::Continue), PumpStatus::Continue);

        events.observe(&WindowEvent::CloseRequested);

        assert_eq!(events.status(PumpStatus::Continue), PumpStatus::Exit);
    }

    #[test]
    fn a_batch_with_no_events_did_no_work_and_a_fetch_alone_is_work() {
        let mut events = PumpEvents::default();
        assert!(!events.did_work());

        events.observe_fetch();

        assert!(events.did_work());
    }
}
