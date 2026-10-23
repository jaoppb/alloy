//! Unit coverage for `core/window`'s `domain/` value objects — no adapter
//! involved. Runs identically under `--no-default-features` and the default
//! `winit-system` build, since none of this depends on `winit`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use window::{
    FrameView, KeyCode, PhysicalPosition, PointerButton, ScaleFactor, SurfaceSize,
    WindowAttributes, WindowError, WindowEvent, WindowId, WindowOperation, WindowTitle,
};

// ---- SurfaceSize ----

#[test]
fn a_zero_width_or_height_surface_is_refused() {
    assert_eq!(SurfaceSize::new(0, 10), None, "zero width has no reading");
    assert_eq!(SurfaceSize::new(10, 0), None, "zero height has no reading");
}

#[test]
fn a_surface_size_round_trips_its_dimensions() {
    let size = SurfaceSize::new(800, 600).expect("800x600 is valid");

    assert_eq!(size.width(), 800);
    assert_eq!(size.height(), 600);
    assert_eq!(size.pixel_count(), Some(480_000));
}

// ---- ScaleFactor ----

#[test]
fn a_scale_factor_refuses_non_finite_or_non_positive_values() {
    for hostile in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
        assert_eq!(
            ScaleFactor::new(hostile),
            None,
            "a scale factor with no correct reading must be refused: {hostile}"
        );
    }
}

#[test]
fn a_legitimate_scale_factor_round_trips() {
    let scale = ScaleFactor::new(2.0).expect("2.0 is a legitimate HiDPI scale");
    assert_eq!(scale.get(), 2.0);
}

// ---- PhysicalPosition ----

#[test]
fn a_physical_position_round_trips_its_coordinates() {
    let position = PhysicalPosition::new(12.5, -3.0);
    assert_eq!(position.x(), 12.5);
    assert_eq!(position.y(), -3.0);
}

// ---- FrameView ----

#[test]
fn a_frame_view_refuses_a_pixel_buffer_of_the_wrong_length() {
    let pixels = [0u32; 3];
    assert_eq!(
        FrameView::new(2, 2, &pixels),
        None,
        "2x2 needs 4 pixels, not 3"
    );
}

#[test]
fn a_frame_view_accepts_a_correctly_sized_buffer() {
    let pixels = [0xFFFF_FFFFu32; 4];
    let frame = FrameView::new(2, 2, &pixels).expect("2x2 needs exactly 4 pixels");

    assert_eq!(frame.width(), 2);
    assert_eq!(frame.height(), 2);
    assert_eq!(frame.pixels(), &pixels);
}

#[test]
fn an_empty_frame_view_is_valid() {
    let pixels: [u32; 0] = [];
    let frame = FrameView::new(0, 0, &pixels).expect("0x0 needs exactly 0 pixels");
    assert_eq!(frame.pixels().len(), 0);
}

// ---- WindowAttributes / WindowTitle / WindowId ----

#[test]
fn window_attributes_round_trip_title_and_size() {
    let size = SurfaceSize::new(1024, 768).expect("1024x768 is valid");
    let attrs = WindowAttributes::new("Alloy", size);

    assert_eq!(attrs.title(), &WindowTitle::from("Alloy"));
    assert_eq!(attrs.initial_size(), size);
}

#[test]
fn a_window_id_round_trips_its_raw_value() {
    let id = WindowId::from_raw(42);
    assert_eq!(id.into_raw(), 42);
}

#[test]
fn distinct_window_ids_are_not_equal() {
    assert_ne!(WindowId::from_raw(1), WindowId::from_raw(2));
}

// ---- WindowError / WindowOperation ----

#[test]
fn no_window_yet_names_the_operation_that_was_attempted() {
    let error = WindowError::no_window_yet(WindowOperation::PumpEvents);
    assert!(matches!(
        error,
        WindowError::NoWindowYet {
            operation: WindowOperation::PumpEvents
        }
    ));
}

#[test]
fn operation_failed_carries_the_window_and_the_operation() {
    let window = WindowId::from_raw(7);
    let error = WindowError::operation_failed(window, WindowOperation::Present, "surface lost");

    let WindowError::OperationFailed {
        window: carried_window,
        operation,
        ..
    } = error
    else {
        panic!("expected OperationFailed");
    };
    assert_eq!(carried_window, window);
    assert_eq!(operation, WindowOperation::Present);
}

// ---- KeyCode ----

#[test]
fn distinct_named_key_codes_are_not_equal() {
    assert_ne!(KeyCode::KEY_A, KeyCode::KEY_B);
    assert_ne!(KeyCode::UNIDENTIFIED, KeyCode::ENTER);
}

#[test]
fn a_key_code_round_trips_a_raw_identifier() {
    let code = KeyCode::from_raw(999);
    assert_eq!(code.raw(), 999);
}

// ---- WindowEvent / PointerButton ----

#[test]
fn pointer_button_other_carries_its_backend_index() {
    let button = PointerButton::Other(9);
    assert_eq!(button, PointerButton::Other(9));
    assert_ne!(button, PointerButton::Other(8));
}

#[test]
fn a_resized_event_carries_the_reported_size() {
    let size = SurfaceSize::new(640, 480).expect("640x480 is valid");
    let event = WindowEvent::Resized(size);

    let WindowEvent::Resized(reported) = event else {
        panic!("expected Resized");
    };
    assert_eq!(reported, size);
}

// ---- Attributes and errors ----

#[test]
fn window_attributes_carry_a_title_and_size_and_a_title_prints_verbatim() {
    let size = SurfaceSize::new(320, 200).unwrap();
    let attributes = WindowAttributes::new("Alloy", size);
    assert_eq!(attributes.title().as_str(), "Alloy");
    assert_eq!(attributes.initial_size(), size);
    assert_eq!(WindowTitle::from(String::from("Tab")).to_string(), "Tab");
    assert_eq!(WindowTitle::default().as_str(), "");
}

#[test]
fn a_window_id_round_trips_and_prints_with_its_number() {
    let identifier = WindowId::from_raw(7);
    assert_eq!(identifier.into_raw(), 7);
    assert_eq!(identifier.to_string(), "window#7");
}

#[test]
fn window_errors_name_the_operation_and_the_cause() {
    let operations = [
        (WindowOperation::CreateWindow, "create_window"),
        (WindowOperation::PumpEvents, "pump_events"),
        (WindowOperation::Present, "present"),
    ];
    for (operation, name) in operations {
        assert_eq!(operation.name(), name);
        assert_eq!(operation.to_string(), name);
    }
    assert_eq!(
        WindowError::no_window_yet(WindowOperation::Present).to_string(),
        "present was attempted before any window was created"
    );
    assert_eq!(
        WindowError::creation_failed("no display").to_string(),
        "the window could not be created — no display"
    );
    assert_eq!(
        WindowError::operation_failed(WindowId::from_raw(2), WindowOperation::Present, "lost")
            .to_string(),
        "present failed for window#2 — lost"
    );
}
