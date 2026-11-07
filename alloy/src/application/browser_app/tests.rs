//! State [`BrowserApp`] keeps between frames.
#![allow(clippy::unwrap_used)]

use ui::KeymapBinding;
use window::{InputText, KeyCode, KeyEvent, KeyState, LogicalKey, Modifiers, SurfaceSize};

use super::test_support::{FakeWeb, app_over};
use crate::application::ports::KeymapSource;
use crate::composition::initial_window_attributes;

#[test]
fn a_fresh_app_has_the_default_ui_render_tree_hierarchy() {
    let attributes = initial_window_attributes().unwrap();
    let app = app_over(FakeWeb::new(), attributes.initial_size());
    let tree = &app.ui_tree;
    assert_eq!(tree.root().kind(), &ui::UiNodeKind::Window);
    assert_eq!(tree.chrome_height(), ui::ChromeHeight::DEFAULT);
}

#[test]
fn resizing_below_chrome_height_updates_ui_tree_to_fallback() {
    let attributes = initial_window_attributes().unwrap();
    let mut app = app_over(FakeWeb::new(), attributes.initial_size());
    let small_size = SurfaceSize::new(800, 30).unwrap();
    app.resize(small_size);

    assert_eq!(app.ui_tree.viewport(), small_size);
    assert_eq!(app.ui_tree.content_size().unwrap(), None);
    assert_eq!(
        app.ui_tree
            .hit_test(window::PhysicalPosition::new(20.0, 20.0)),
        None
    );

    // Resizing back to a valid viewport restores the full hierarchy
    let normal_size = SurfaceSize::new(800, 600).unwrap();
    app.resize(normal_size);
    assert_eq!(app.ui_tree.viewport(), normal_size);
    assert!(app.ui_tree.content_size().unwrap().is_some());
    assert_eq!(
        app.ui_tree
            .hit_test(window::PhysicalPosition::new(20.0, 20.0)),
        Some(ui::UiHitTarget::Omnibox)
    );
}

struct OneBinding;

impl KeymapSource for OneBinding {
    fn bindings(&self) -> Vec<KeymapBinding> {
        vec![KeymapBinding::new("ctrl+k", "focus_omnibox")]
    }
}

#[test]
fn reloading_the_keymap_makes_a_new_chord_focus_the_omnibox() {
    let attributes = initial_window_attributes().unwrap();
    let mut app = app_over(FakeWeb::new(), attributes.initial_size());
    let letter = InputText::new("k").unwrap();
    let ctrl_k = KeyEvent::new(
        LogicalKey::Character(letter.clone()),
        KeyCode::KEY_K,
        KeyState::Pressed,
    )
    .with_modifiers(Modifiers::NONE.with_control())
    .with_text(letter);

    app.handle_key(&ctrl_k);
    assert!(
        !app.ui.omnibox().is_focused(),
        "ctrl+k is unbound by default"
    );

    app.reload_keymap(&OneBinding);
    app.handle_key(&ctrl_k);

    assert!(app.ui.omnibox().is_focused());
}
