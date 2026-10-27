#![allow(clippy::unwrap_used, clippy::unnecessary_wraps)]

use graphics::{Au, Point, Rect, Size};
use platform::HostOs;
use region::{ContentRegion, ContextId, LayoutEpoch, RegionId};
use window::{
    InputText, KeyCode, KeyEvent, KeyState, LogicalKey, Modifiers, PhysicalPosition, SurfaceSize,
};

use super::{InputRouter, UiController};
use crate::domain::input::{BrowserCommand, FocusTarget, Keymap, KeymapBinding, UiEffect, UiInput};
use crate::domain::omnibox::EditKey;
use crate::domain::render_tree::UiRenderTree;
use crate::domain::theme::OmniboxTheme;

const CTRL: Modifiers = Modifiers::NONE.with_control();
const ALT: Modifiers = Modifiers::NONE.with_alt();
const ALT_GRAPH: Modifiers = Modifiers::NONE.with_alt_graph();
const CTRL_ALT: Modifiers = Modifiers::NONE.with_control().with_alt();

fn text(value: &str) -> InputText {
    InputText::new(value).unwrap()
}

fn key(
    logical: LogicalKey,
    physical: KeyCode,
    typed: Option<&str>,
    modifiers: Modifiers,
    state: KeyState,
) -> KeyEvent {
    let event = KeyEvent::new(logical, physical, state).with_modifiers(modifiers);
    match typed {
        Some(typed) => event.with_text(text(typed)),
        None => event,
    }
}

/// A character key pressed the way `winit` reports it: logical and text agree.
fn typed(value: &str, physical: KeyCode, modifiers: Modifiers) -> KeyEvent {
    let logical = LogicalKey::Character(text(value));
    key(logical, physical, Some(value), modifiers, KeyState::Pressed)
}

fn router(host: HostOs) -> InputRouter {
    InputRouter::new(Keymap::default(), host)
}

fn route(event: &KeyEvent) -> Option<UiInput> {
    router(HostOs::Other).route_key(event, FocusTarget::Omnibox)
}

fn insert(value: &str) -> Option<UiInput> {
    Some(UiInput::Insert(text(value)))
}

fn focus_command() -> Option<UiInput> {
    Some(UiInput::Command(BrowserCommand::FocusOmnibox))
}

#[test]
fn ctrl_l_and_alt_d_resolve_to_the_focus_command_even_though_winit_reports_text() {
    assert_eq!(route(&typed("l", KeyCode::KEY_L, CTRL)), focus_command());
    assert_eq!(route(&typed("d", KeyCode::KEY_D, ALT)), focus_command());
    let super_l = typed("l", KeyCode::KEY_L, Modifiers::NONE.with_super_key());
    assert_eq!(route(&super_l), focus_command());
}

#[test]
fn alt_gr_q_types_an_at_sign() {
    assert_eq!(route(&typed("@", KeyCode::KEY_Q, ALT_GRAPH)), insert("@"));
}

#[test]
fn ctrl_alt_gr_l_stays_bindable() {
    let chord = Modifiers::NONE.with_control().with_alt_graph();
    let load = Keymap::from_bindings([KeymapBinding::new("ctrl+altgr+l", "focus_omnibox")]);
    let router = InputRouter::new(load.into_keymap(), HostOs::Other);
    let event = typed("l", KeyCode::KEY_L, chord);
    assert_eq!(
        router.route_key(&event, FocusTarget::Omnibox),
        focus_command()
    );
}

#[test]
fn windows_ctrl_alt_types_text_even_with_a_ctrl_alt_binding() {
    let load = Keymap::from_bindings([KeymapBinding::new("ctrl+alt+q", "focus_omnibox")]);
    let windows = InputRouter::new(load.into_keymap(), HostOs::Windows);
    let at_sign = typed("@", KeyCode::KEY_Q, CTRL_ALT);
    assert_eq!(
        windows.route_key(&at_sign, FocusTarget::Omnibox),
        insert("@")
    );
}

#[test]
fn ctrl_alt_text_is_dropped_off_windows() {
    assert_eq!(route(&typed("@", KeyCode::KEY_Q, CTRL_ALT)), None);
    let shifted = CTRL_ALT.with_shift();
    let windows = router(HostOs::Windows);
    assert_eq!(
        windows.route_key(&typed("@", KeyCode::KEY_Q, shifted), FocusTarget::Omnibox),
        insert("@"),
        "Shift never blocks",
    );
}

#[test]
fn windows_ctrl_alt_with_super_is_not_alt_gr() {
    let event = typed("@", KeyCode::KEY_Q, CTRL_ALT.with_super_key());
    assert_eq!(
        router(HostOs::Windows).route_key(&event, FocusTarget::Omnibox),
        None
    );
}

#[test]
fn an_unbound_ctrl_k_inserts_nothing() {
    assert_eq!(route(&typed("k", KeyCode::KEY_K, CTRL)), None);
    let super_k = typed("k", KeyCode::KEY_K, Modifiers::NONE.with_super_key());
    assert_eq!(route(&super_k), None);
    assert_eq!(route(&typed("k", KeyCode::KEY_K, ALT)), None);
}

#[test]
fn shift_never_blocks_typing() {
    let shifted = typed("A", KeyCode::KEY_A, Modifiers::NONE.with_shift());
    assert_eq!(route(&shifted), insert("A"));
}

#[test]
fn macos_option_composes_text_while_cmd_is_a_command() {
    let mac = router(HostOs::MacOs);
    let option_e = typed("é", KeyCode::KEY_E, ALT);
    assert_eq!(mac.route_key(&option_e, FocusTarget::Omnibox), insert("é"));
    let cmd_l = typed("l", KeyCode::KEY_L, Modifiers::NONE.with_super_key());
    assert_eq!(mac.route_key(&cmd_l, FocusTarget::Omnibox), focus_command());
    let cmd_k = typed("k", KeyCode::KEY_K, Modifiers::NONE.with_super_key());
    assert_eq!(mac.route_key(&cmd_k, FocusTarget::Omnibox), None);
}

#[test]
fn macos_option_backspace_is_not_an_edit() {
    let backspace = key(
        LogicalKey::Backspace,
        KeyCode::BACKSPACE,
        None,
        ALT,
        KeyState::Pressed,
    );
    assert_eq!(
        router(HostOs::MacOs).route_key(&backspace, FocusTarget::Omnibox),
        None
    );
}

#[test]
fn a_release_never_produces_input() {
    let release = |event: KeyEvent| {
        key(
            event.logical().clone(),
            event.physical(),
            event.text().map(InputText::as_str),
            event.modifiers(),
            KeyState::Released,
        )
    };
    assert_eq!(route(&release(typed("l", KeyCode::KEY_L, CTRL))), None);
    assert_eq!(
        route(&release(typed("a", KeyCode::KEY_A, Modifiers::NONE))),
        None
    );
    let backspace = key(
        LogicalKey::Backspace,
        KeyCode::BACKSPACE,
        None,
        Modifiers::NONE,
        KeyState::Released,
    );
    assert_eq!(route(&backspace), None);
}

#[test]
fn a_held_focus_chord_fires_once_but_a_held_edit_key_repeats() {
    let repeat = |logical, physical, typed, modifiers| {
        key(logical, physical, typed, modifiers, KeyState::Repeated)
    };
    let ctrl_l = repeat(
        LogicalKey::Character(text("l")),
        KeyCode::KEY_L,
        Some("l"),
        CTRL,
    );
    assert_eq!(route(&ctrl_l), None);
    let backspace = repeat(
        LogicalKey::Backspace,
        KeyCode::BACKSPACE,
        None,
        Modifiers::NONE,
    );
    assert_eq!(route(&backspace), Some(UiInput::Edit(EditKey::Backspace)));
    let letter = repeat(
        LogicalKey::Character(text("a")),
        KeyCode::KEY_A,
        Some("a"),
        Modifiers::NONE,
    );
    assert_eq!(route(&letter), insert("a"));
}

#[test]
fn named_editing_keys_edit_unless_a_command_modifier_is_held() {
    let backspace = |modifiers| {
        key(
            LogicalKey::Backspace,
            KeyCode::BACKSPACE,
            None,
            modifiers,
            KeyState::Pressed,
        )
    };
    assert_eq!(
        route(&backspace(Modifiers::NONE)),
        Some(UiInput::Edit(EditKey::Backspace))
    );
    assert_eq!(
        route(&backspace(Modifiers::NONE.with_shift())),
        Some(UiInput::Edit(EditKey::Backspace))
    );
    assert_eq!(route(&backspace(CTRL)), None);
    assert_eq!(route(&backspace(ALT)), None);
    assert_eq!(
        route(&backspace(ALT_GRAPH)),
        Some(UiInput::Edit(EditKey::Backspace))
    );
}

#[test]
fn typing_and_editing_reach_only_a_focused_omnibox() {
    let router = router(HostOs::Other);
    let letter = typed("a", KeyCode::KEY_A, Modifiers::NONE);
    assert_eq!(router.route_key(&letter, FocusTarget::Content), None);
    assert_eq!(router.route_text(text("a"), FocusTarget::Content), None);
    assert_eq!(
        router.route_text(text("a"), FocusTarget::Omnibox),
        insert("a")
    );
    let ctrl_l = typed("l", KeyCode::KEY_L, CTRL);
    assert_eq!(
        router.route_key(&ctrl_l, FocusTarget::Content),
        focus_command()
    );
}

#[test]
fn the_controller_focuses_types_and_ignores_unfocused_typing() {
    let mut controller = UiController::new(Keymap::default(), HostOs::Other);
    assert_eq!(controller.focus(), FocusTarget::Content);
    assert_eq!(
        controller.handle_key(&typed("a", KeyCode::KEY_A, Modifiers::NONE)),
        UiEffect::None
    );
    assert_eq!(
        controller.handle_key(&typed("l", KeyCode::KEY_L, CTRL)),
        UiEffect::ChromeDamaged
    );
    assert_eq!(controller.focus(), FocusTarget::Omnibox);
    assert_eq!(
        controller.handle_key(&typed("x", KeyCode::KEY_X, CTRL)),
        UiEffect::None
    );
    controller.handle_key(&typed("q", KeyCode::KEY_Q, Modifiers::NONE));
    assert_eq!(controller.omnibox().text(), "q");
    controller.handle_text_committed(text("é"));
    assert_eq!(controller.omnibox().text(), "qé");
}

// ---- pointer ----

fn link_region() -> ContentRegion {
    let au = |pixels| Au::from_whole_px(pixels).unwrap();
    let bounds = Rect::new(
        Point::new(au(10), au(10)),
        Size::new(au(20), au(20)).unwrap(),
    );
    let id = RegionId::new(ContextId::ROOT, LayoutEpoch::INITIAL.next(), 0);
    ContentRegion::new(id, bounds)
}

fn tree_with(region: ContentRegion) -> UiRenderTree {
    let size = SurfaceSize::new(800, 600).unwrap();
    UiRenderTree::build(size, &OmniboxTheme::DEFAULT, [region]).unwrap()
}

fn chrome_position() -> PhysicalPosition {
    PhysicalPosition::new(20.0, 20.0)
}

fn content_position(document_y: f64) -> PhysicalPosition {
    let chrome = f64::from(OmniboxTheme::DEFAULT.chrome_height().to_px().get());
    PhysicalPosition::new(20.0, chrome + document_y)
}

#[test]
fn clicking_the_chrome_focuses_the_omnibox() {
    let mut controller = UiController::new(Keymap::default(), HostOs::Other);
    let tree = tree_with(link_region());
    assert_eq!(
        controller.handle_click(&tree, chrome_position()),
        UiEffect::ChromeDamaged
    );
    assert_eq!(controller.focus(), FocusTarget::Omnibox);
}

#[test]
fn clicking_a_link_asks_the_browser_to_follow_it_and_takes_focus_from_the_omnibox() {
    let region = link_region();
    let tree = tree_with(region);
    let mut controller = UiController::new(Keymap::default(), HostOs::Other);
    controller.handle_click(&tree, chrome_position());
    let effect = controller.handle_click(&tree, content_position(20.0));
    assert_eq!(effect, UiEffect::FollowLink(region.id()));
    assert_eq!(controller.focus(), FocusTarget::Content);
}

#[test]
fn clicking_empty_page_content_only_moves_focus() {
    let tree = tree_with(link_region());
    let mut controller = UiController::new(Keymap::default(), HostOs::Other);
    assert_eq!(
        controller.handle_click(&tree, content_position(300.0)),
        UiEffect::None
    );
    controller.handle_click(&tree, chrome_position());
    let effect = controller.handle_click(&tree, content_position(300.0));
    assert_eq!(effect, UiEffect::ChromeDamaged);
    assert_eq!(controller.focus(), FocusTarget::Content);
}

#[test]
fn enter_in_the_omnibox_navigates_and_returns_focus_to_the_page() {
    let mut controller = UiController::new(Keymap::default(), HostOs::Other);
    controller.handle_key(&typed("l", KeyCode::KEY_L, CTRL));
    controller.handle_text_committed(text("example.com"));
    let enter = key(
        LogicalKey::Enter,
        KeyCode::ENTER,
        None,
        Modifiers::NONE,
        KeyState::Pressed,
    );
    assert!(matches!(
        controller.handle_key(&enter),
        UiEffect::Navigate(_)
    ));
    assert_eq!(controller.focus(), FocusTarget::Content);
}
