use network::Url;
use window::{InputText, LogicalKey};

use super::buffer::{CursorPosition, OmniboxBuffer};
use super::controller::{OmniboxController, OmniboxEffect, OmniboxInput};
use super::gap_buffer::GapBuffer;
use super::state::OmniboxState;

#[test]
fn cursor_position_value_object() {
    let position = CursorPosition::new(42);
    assert_eq!(position.byte_offset(), 42);
    assert_eq!(format!("{position}"), "cursor@42");
    assert_eq!(CursorPosition::ZERO.byte_offset(), 0);
}

#[test]
fn empty_buffer_invariants() {
    let mut buffer = OmniboxBuffer::new();
    assert!(buffer.is_empty());
    assert_eq!(buffer.len(), 0);
    assert_eq!(buffer.char_count(), 0);
    assert_eq!(buffer.cursor_position(), CursorPosition::ZERO);
    assert_eq!(buffer.cursor_char_index(), 0);
    assert_eq!(buffer.text(), "");
    assert_eq!(buffer.text_before_cursor(), "");
    assert_eq!(buffer.text_after_cursor(), "");

    buffer.backspace();
    assert_eq!(buffer.len(), 0);

    buffer.delete();
    assert_eq!(buffer.len(), 0);

    buffer.move_left();
    assert_eq!(buffer.cursor_position(), CursorPosition::ZERO);

    buffer.move_right();
    assert_eq!(buffer.cursor_position(), CursorPosition::ZERO);
}

#[test]
fn ascii_editing_and_cursor_navigation() {
    let mut buffer = OmniboxBuffer::from_text("hello");
    assert_eq!(buffer.len(), 5);
    assert_eq!(buffer.char_count(), 5);
    assert_eq!(buffer.cursor_position(), CursorPosition::new(5));
    assert_eq!(buffer.text_before_cursor(), "hello");
    assert_eq!(buffer.text_after_cursor(), "");

    buffer.move_left();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(4));
    assert_eq!(buffer.text_before_cursor(), "hell");
    assert_eq!(buffer.text_after_cursor(), "o");

    buffer.insert_text("X");
    assert_eq!(buffer.text(), "hellXo");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(5));

    buffer.backspace();
    assert_eq!(buffer.text(), "hello");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(4));

    buffer.delete();
    assert_eq!(buffer.text(), "hell");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(4));

    buffer.move_start();
    assert_eq!(buffer.cursor_position(), CursorPosition::ZERO);
    assert_eq!(buffer.text_before_cursor(), "");
    assert_eq!(buffer.text_after_cursor(), "hell");

    buffer.delete();
    assert_eq!(buffer.text(), "ell");
    assert_eq!(buffer.cursor_position(), CursorPosition::ZERO);

    buffer.move_end();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(3));

    buffer.set_text("world");
    assert_eq!(buffer.text(), "world");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(5));

    buffer.clear();
    assert!(buffer.is_empty());
    assert_eq!(buffer.cursor_position(), CursorPosition::ZERO);
}

#[test]
fn multibyte_latin_accents_utf8() {
    let mut buffer = OmniboxBuffer::from_text("olá mundo");
    assert_eq!(buffer.char_count(), 9);
    assert_eq!(buffer.len(), 10);
    assert_eq!(buffer.cursor_position(), CursorPosition::new(10));

    buffer.move_start();
    buffer.move_right();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(1));
    buffer.move_right();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(2));

    buffer.move_right();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(4));
    assert_eq!(buffer.text_before_cursor(), "olá");
    assert_eq!(buffer.text_after_cursor(), " mundo");

    buffer.backspace();
    assert_eq!(buffer.text(), "ol mundo");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(2));

    buffer.insert_text("á");
    assert_eq!(buffer.text(), "olá mundo");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(4));

    buffer.move_left();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(2));
    buffer.delete();
    assert_eq!(buffer.text(), "ol mundo");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(2));
}

#[test]
fn multibyte_cjk_characters_utf8() {
    let mut buffer = OmniboxBuffer::from_text("日本語");
    assert_eq!(buffer.char_count(), 3);
    assert_eq!(buffer.len(), 9);
    assert_eq!(buffer.cursor_position(), CursorPosition::new(9));

    buffer.move_left();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(6));
    assert_eq!(buffer.text_before_cursor(), "日本");
    assert_eq!(buffer.text_after_cursor(), "語");

    buffer.move_left();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(3));

    buffer.move_left();
    assert_eq!(buffer.cursor_position(), CursorPosition::ZERO);

    buffer.delete();
    assert_eq!(buffer.text(), "本語");
    assert_eq!(buffer.len(), 6);
    assert_eq!(buffer.cursor_position(), CursorPosition::ZERO);

    buffer.move_end();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(6));
    buffer.backspace();
    assert_eq!(buffer.text(), "本");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(3));
}

#[test]
fn multibyte_emoji_characters_utf8() {
    let mut buffer = OmniboxBuffer::from_text("🦀🚀");
    assert_eq!(buffer.char_count(), 2);
    assert_eq!(buffer.len(), 8);
    assert_eq!(buffer.cursor_position(), CursorPosition::new(8));

    buffer.move_left();
    assert_eq!(buffer.cursor_position(), CursorPosition::new(4));
    assert_eq!(buffer.text_before_cursor(), "🦀");
    assert_eq!(buffer.text_after_cursor(), "🚀");

    buffer.backspace();
    assert_eq!(buffer.text(), "🚀");
    assert_eq!(buffer.cursor_position(), CursorPosition::ZERO);

    buffer.insert_text("✨");
    assert_eq!(buffer.text(), "✨🚀");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(3));

    buffer.delete();
    assert_eq!(buffer.text(), "✨");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(3));
}

#[test]
fn newline_sanitization_in_insertion() {
    let mut buffer = OmniboxBuffer::new();
    buffer.insert_text("https://example.com\r\n/path\n");
    assert_eq!(buffer.text(), "https://example.com/path");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(24));
}

#[test]
fn single_line_sanitization_in_constructor_and_set_text() {
    let mut buffer = OmniboxBuffer::from_text("https://example.com\r\n/path\n");
    assert_eq!(buffer.text(), "https://example.com/path");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(24));

    buffer.set_text("first\r\nsecond\nthird");
    assert_eq!(buffer.text(), "firstsecondthird");
    assert_eq!(buffer.cursor_position(), CursorPosition::new(16));
}

#[test]
fn gap_buffer_capacity_expansion_and_slice_integrity() {
    let mut gap_buf = GapBuffer::new();
    assert!(gap_buf.is_empty());
    assert_eq!(gap_buf.len(), 0);

    // Insert text larger than DEFAULT_GAP_CAPACITY (64 bytes) to trigger growth
    let large_input = "a".repeat(100);
    gap_buf.insert_text(&large_input);
    assert_eq!(gap_buf.len(), 100);
    assert_eq!(gap_buf.char_count(), 100);
    assert_eq!(gap_buf.cursor_byte_offset(), 100);
    assert_eq!(gap_buf.text_before_cursor(), large_input);
    assert_eq!(gap_buf.text_after_cursor(), "");

    gap_buf.move_left();
    assert_eq!(gap_buf.cursor_byte_offset(), 99);
    assert_eq!(gap_buf.text_before_cursor(), "a".repeat(99));
    assert_eq!(gap_buf.text_after_cursor(), "a");
    assert_eq!(gap_buf.text(), large_input);

    gap_buf.insert_text("XYZ");
    assert_eq!(gap_buf.len(), 103);
    assert_eq!(gap_buf.cursor_byte_offset(), 102);
    assert_eq!(gap_buf.text_after_cursor(), "a");

    gap_buf.move_start();
    assert_eq!(gap_buf.cursor_byte_offset(), 0);
    assert_eq!(gap_buf.text_before_cursor(), "");
    assert_eq!(gap_buf.text_after_cursor().len(), 103);

    gap_buf.move_end();
    assert_eq!(gap_buf.cursor_byte_offset(), 103);
    assert_eq!(gap_buf.text_before_cursor().len(), 103);
    assert_eq!(gap_buf.text_after_cursor(), "");
}

#[test]
fn omnibox_state_lifecycle() {
    let mut state = OmniboxState::new();
    assert!(!state.is_focused());
    assert!(!state.is_all_selected());
    assert_eq!(state.active_url(), None);
    assert_eq!(state.text(), "");

    state.focus();
    assert!(state.is_focused());

    state.insert_text("rust");
    assert_eq!(state.text(), "rust");
    assert_eq!(state.cursor_position(), CursorPosition::new(4));

    state.select_all();
    assert!(state.is_all_selected());

    state.insert_text("a");
    assert_eq!(state.text(), "a");
    assert!(!state.is_all_selected());
    assert_eq!(state.cursor_position(), CursorPosition::new(1));

    state.select_all();
    state.backspace();
    assert_eq!(state.text(), "");
    assert!(!state.is_all_selected());

    state.insert_text("alloy");
    state.select_all();
    state.delete();
    assert_eq!(state.text(), "");
    assert!(!state.is_all_selected());

    state.insert_text("sample");
    state.select_all();
    state.move_left();
    assert!(!state.is_all_selected());
    assert_eq!(state.cursor_position(), CursorPosition::ZERO);

    state.select_all();
    state.move_right();
    assert!(!state.is_all_selected());
    assert_eq!(state.cursor_position(), CursorPosition::new(6));

    state.blur();
    assert!(!state.is_focused());
    assert!(!state.is_all_selected());
}

#[test]
fn omnibox_state_with_and_reset_to_url() {
    let url = Url::parse("https://example.com/").expect("valid url");
    let mut state = OmniboxState::with_url(url.clone());

    assert_eq!(state.active_url(), Some(&url));
    assert_eq!(state.text(), "https://example.com/");
    assert!(!state.is_focused());

    state.focus();
    state.select_all();
    state.insert_text("search query");
    assert_eq!(state.text(), "search query");
    assert!(state.is_focused());

    state.cancel_editing();
    assert_eq!(state.text(), "https://example.com/");
    assert!(!state.is_focused());

    let other_url = Url::parse("https://github.com/jaoppb/alloy").expect("valid url");
    state.focus();
    state.reset_to_url(&other_url);
    assert_eq!(state.active_url(), Some(&other_url));
    assert_eq!(state.text(), "https://github.com/jaoppb/alloy");
    assert!(!state.is_focused());
    assert!(!state.is_all_selected());
}

#[test]
fn controller_initialization_and_accessors() {
    let controller = OmniboxController::new();
    assert!(!controller.is_focused());
    assert!(!controller.is_all_selected());
    assert_eq!(controller.text(), "");
    assert_eq!(controller.state().active_url(), None);

    let url = Url::parse("https://example.com/page").expect("valid url");
    let mut controller = OmniboxController::with_url(url);
    assert!(!controller.is_focused());
    assert_eq!(controller.text(), "https://example.com/page");

    let new_url = Url::parse("https://example.org/").expect("valid url");
    controller.reset_to_url(&new_url);
    assert_eq!(controller.text(), "https://example.org/");
    assert_eq!(controller.state().active_url(), Some(&new_url));
}

#[test]
fn controller_focus_and_text_input_lifecycle() {
    let mut controller = OmniboxController::new();

    let effect = controller.handle_text_input("ignored");
    assert_eq!(effect, OmniboxEffect::None);
    assert_eq!(controller.text(), "");

    let effect = controller.focus();
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert!(controller.is_focused());
    assert!(controller.is_all_selected());

    let effect = controller.handle_text_input("hello");
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert_eq!(controller.text(), "hello");
    assert!(!controller.is_all_selected());
}

#[test]
fn controller_escape_reverts_to_active_url() {
    let url = Url::parse("https://example.com/").expect("valid url");
    let mut controller = OmniboxController::with_url(url);

    let effect = controller.handle_escape();
    assert_eq!(effect, OmniboxEffect::None);

    controller.focus();
    controller.handle_text_input("edits");
    assert_eq!(controller.text(), "edits");

    let effect = controller.handle_escape();
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert!(!controller.is_focused());
    assert_eq!(controller.text(), "https://example.com/");
}

#[test]
fn controller_enter_classifies_and_navigates() {
    let mut controller = OmniboxController::new();

    let effect = controller.handle_enter();
    assert_eq!(effect, OmniboxEffect::None);

    controller.focus();
    controller.handle_text_input("rust-lang.org");

    let effect = controller.handle_enter();
    assert!(!controller.is_focused());
    let expected_url = Url::parse("https://rust-lang.org/").expect("valid url");
    assert_eq!(effect, OmniboxEffect::Navigate(expected_url));
}

#[test]
fn controller_control_key_handling() {
    let mut controller = OmniboxController::new();

    let effect = controller.handle_edit_key(&LogicalKey::Backspace);
    assert_eq!(effect, OmniboxEffect::None);

    controller.focus();
    controller.handle_text_input("cat");
    assert_eq!(controller.text(), "cat");

    let effect = controller.handle_edit_key(&LogicalKey::Backspace);
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert_eq!(controller.text(), "ca");

    let effect = controller.handle_edit_key(&LogicalKey::ArrowLeft);
    assert_eq!(effect, OmniboxEffect::Redraw);

    controller.handle_text_input("o");
    assert_eq!(controller.text(), "coa");

    let effect = controller.handle_edit_key(&LogicalKey::Home);
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert_eq!(controller.state().cursor_position(), CursorPosition::ZERO);

    let effect = controller.handle_edit_key(&LogicalKey::End);
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert_eq!(controller.state().cursor_position(), CursorPosition::new(3));

    let effect = controller.handle_edit_key(&LogicalKey::Delete);
    assert_eq!(effect, OmniboxEffect::Redraw);
}

#[test]
fn controller_handle_input_routes_all_variants() {
    let mut controller = OmniboxController::new();

    let effect = controller.handle_input(&OmniboxInput::Focus);
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert!(controller.is_focused());

    let effect = controller.handle_input(&OmniboxInput::Insert(InputText::new("query").unwrap()));
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert_eq!(controller.text(), "query");

    let effect = controller.handle_input(&OmniboxInput::Edit(LogicalKey::Backspace));
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert_eq!(controller.text(), "quer");

    let effect = controller.handle_input(&OmniboxInput::Edit(LogicalKey::Escape));
    assert_eq!(effect, OmniboxEffect::Redraw);
    assert!(!controller.is_focused());

    controller.handle_input(&OmniboxInput::Focus);
    let effect = controller.handle_input(&OmniboxInput::Edit(LogicalKey::Enter));
    let expected_url =
        Url::parse("https://duckduckgo.com/?q=quer").expect("valid search destination url");
    assert_eq!(effect, OmniboxEffect::Navigate(expected_url));
}
