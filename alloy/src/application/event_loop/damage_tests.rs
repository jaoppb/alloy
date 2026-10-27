//! What each damage signal re-rasters and what it leaves alone (#108, #116, #118).
//!
//! Events are the v4 shape (`Key` carrying `text`), and the assertions read the
//! pixels the presenter actually received, not the omnibox state — the gap that
//! let #116 ship.
#![allow(clippy::unwrap_used)]

use ui::OmniboxTheme;
use window::{
    HeadlessWindowSystem, InputText, KeyCode, KeyEvent, KeyState, LogicalKey, Modifiers,
    RecordingPresenter, SurfaceSize, WindowEvent,
};

use super::pump_once;
use super::test_support::{TestSession, loaded_window, pump};

/// The rows the chrome strip covers, from the same theme the painter uses.
fn chrome_rows() -> usize {
    let whole_pixels = OmniboxTheme::DEFAULT
        .chrome_height()
        .to_au()
        .raw()
        .checked_div(graphics::AU_PER_PX)
        .unwrap();
    usize::try_from(whole_pixels).unwrap()
}

/// A copy of the last presented frame: `(width, height, pixels)`.
fn presented(presenter: &RecordingPresenter) -> (usize, usize, Vec<u32>) {
    let frame = presenter.last_frame().expect("a frame was presented");
    (
        usize::try_from(frame.width()).unwrap(),
        usize::try_from(frame.height()).unwrap(),
        frame.pixels().to_vec(),
    )
}

fn type_text(system: &mut HeadlessWindowSystem, text: &str) {
    for character in text.chars() {
        let character = InputText::new(character.to_string()).unwrap();
        let event = KeyEvent::new(
            LogicalKey::Character(character.clone()),
            KeyCode::UNIDENTIFIED,
            KeyState::Pressed,
        )
        .with_text(character);
        system.schedule(WindowEvent::Key(event));
    }
}

fn press_ctrl_l(system: &mut HeadlessWindowSystem) {
    let letter = InputText::new("l").unwrap();
    let pressed = KeyEvent::new(
        LogicalKey::Character(letter.clone()),
        KeyCode::KEY_L,
        KeyState::Pressed,
    )
    .with_modifiers(Modifiers::NONE.with_control())
    .with_text(letter);
    system.schedule(WindowEvent::Key(pressed));
}

/// A page laid out and presented once, the omnibox untouched.
fn presented_page() -> (HeadlessWindowSystem, RecordingPresenter, TestSession) {
    let (mut system, mut presenter, mut session) = loaded_window();
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(session.stats().relayouts, 1);
    (system, presenter, session)
}

/// Asserts `after` differs from `before` in the chrome rows and nowhere else.
fn assert_only_chrome_changed(before: &(usize, usize, Vec<u32>), after: &(usize, usize, Vec<u32>)) {
    assert_eq!((before.0, before.1), (after.0, after.1));
    let boundary = chrome_rows().saturating_mul(before.0);
    assert_ne!(
        before.2[..boundary],
        after.2[..boundary],
        "the chrome rows must show the change"
    );
    assert_eq!(
        before.2[boundary..],
        after.2[boundary..],
        "the content rows must be untouched"
    );
}

#[test]
fn typing_in_the_omnibox_changes_the_presented_chrome_rows_and_not_the_content() {
    let (mut system, mut presenter, mut session) = presented_page();
    press_ctrl_l(&mut system);
    pump(&mut system, &mut presenter, &mut session);
    let before = presented(&presenter);

    type_text(&mut system, "abc");
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(session.omnibox().text(), "abc");
    assert_only_chrome_changed(&before, &presented(&presenter));
    assert_eq!(
        session.stats().relayouts,
        1,
        "typing must never relayout the document"
    );
}

#[test]
fn focusing_the_omnibox_changes_the_presented_chrome_rows() {
    let (mut system, mut presenter, mut session) = presented_page();
    let before = presented(&presenter);

    press_ctrl_l(&mut system);
    pump(&mut system, &mut presenter, &mut session);

    assert!(session.omnibox().is_focused());
    assert_only_chrome_changed(&before, &presented(&presenter));
    assert_eq!(session.stats().relayouts, 1);
}

#[test]
fn many_chrome_damages_in_one_pump_cost_one_present() {
    let (mut system, mut presenter, mut session) = presented_page();
    press_ctrl_l(&mut system);
    pump(&mut system, &mut presenter, &mut session);
    let presents_before = presenter.present_count();

    type_text(&mut system, "a long run of typed characters");
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(
        presenter.present_count(),
        presents_before + 1,
        "thirty keystrokes coalesce into a single chrome raster and present"
    );
    assert_eq!(session.stats().relayouts, 1);
}

#[test]
fn a_viewport_change_and_typing_in_one_pump_cost_one_relayout() {
    let (mut system, mut presenter, mut session) = presented_page();
    press_ctrl_l(&mut system);
    pump(&mut system, &mut presenter, &mut session);

    system.schedule(WindowEvent::Resized(SurfaceSize::new(640, 480).unwrap()));
    type_text(&mut system, "abc");
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(session.stats().relayouts, 2);
    let (width, height, _) = presented(&presenter);
    assert_eq!((width, height), (640, 480));
}

#[test]
fn a_viewport_no_taller_than_the_chrome_keeps_the_session_alive() {
    let chrome = u32::try_from(chrome_rows()).unwrap();
    for height in [1, chrome, chrome + 1] {
        let (mut system, mut presenter, mut session) = presented_page();
        let small = SurfaceSize::new(800, height).unwrap();

        system.schedule(WindowEvent::Resized(small));
        pump(&mut system, &mut presenter, &mut session);

        let (width, presented_height, pixels) = presented(&presenter);
        assert_eq!(
            (width, presented_height),
            (800, usize::try_from(height).unwrap())
        );
        assert_eq!(pixels.len(), width * presented_height);
    }
}

#[test]
fn growing_back_from_a_chrome_only_viewport_renders_the_page_again() {
    let (mut system, mut presenter, mut session) = presented_page();
    let chrome_only = presented(&presenter).2[..chrome_rows() * 800].to_vec();

    system.schedule(WindowEvent::Resized(SurfaceSize::new(800, 30).unwrap()));
    pump(&mut system, &mut presenter, &mut session);
    assert_eq!(
        session.stats().relayouts,
        1,
        "a viewport with no content rows has nothing to lay out"
    );

    system.schedule(WindowEvent::Resized(SurfaceSize::new(800, 600).unwrap()));
    pump(&mut system, &mut presenter, &mut session);

    assert_eq!(session.stats().relayouts, 2);
    let (width, height, pixels) = presented(&presenter);
    assert_eq!((width, height), (800, 600));
    assert_eq!(pixels[..chrome_only.len()], chrome_only[..]);
}

#[test]
fn an_idle_session_neither_presents_nor_rearms_the_redraw() {
    let (mut system, mut presenter, mut session) = presented_page();
    // The first present re-armed one redraw (Wayland); that re-blit settles it.
    pump(&mut system, &mut presenter, &mut session);
    let presents_settled = presenter.present_count();

    for cycle in 0..5 {
        let (_, did_work) = pump_once(&mut system, &mut presenter, &mut session).unwrap();
        assert!(!did_work, "idle cycle {cycle} must let the loop sleep");
    }

    assert_eq!(presenter.present_count(), presents_settled);
    assert_eq!(session.stats().relayouts, 1);
}
