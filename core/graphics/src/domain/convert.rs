//! The one place in this crate where a float becomes an integer.
//!
//! `std` bridges neither `f32 → i32` nor `f32 → u8`: there is no `TryFrom` for
//! either direction, so `as` is the only conversion the language offers. Rather
//! than scatter that carve-out across every value object that needs it, the
//! crate concentrates it here — one file, three functions, each of which clamps
//! into the target range *before* converting, so no call can truncate or wrap.
//!
//! This mirrors the existing house carve-out at
//! `core/engine/src/domain/value.rs:85`, which allows the same lints for the
//! same reason. Concentrating it in a single auditable file is what keeps the
//! `ADR-0017` allow budget legible.

/// Rounds `value` to the nearest integer, clamps it into `[minimum, maximum]`,
/// and narrows it to an `i32`.
///
/// `value` must already be finite: [`crate::Au::from_px`] is the only caller and
/// rejects `NaN` and `±inf` before reaching here.
// `std` offers no `TryFrom<f32> for i32`. The value is clamped into `i32` range
// on the line above the cast, so the cast cannot truncate.
#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
pub const fn round_and_clamp_to_i32(value: f32, minimum: i32, maximum: i32) -> i32 {
    let bounded = value.round().clamp(to_f32(minimum), to_f32(maximum));
    bounded as i32
}

/// Clamps `value` to the unit interval and scales it onto `[0, 255]`.
// `std` offers no `TryFrom<f32> for u8`. The input is clamped to `[0.0, 1.0]`
// and scaled by `u8::MAX`, so the rounded result is always in range.
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn unit_interval_to_u8(value: f32) -> u8 {
    let scaled = value.clamp(0.0, 1.0) * f32::from(u8::MAX);
    scaled.round() as u8
}

/// Widens an `i32` to the nearest `f32`.
///
/// Lossy past 2^24 by design: the only callers are diagnostics and the
/// author-facing `Au → Px` round trip, never geometry a golden image compares.
// `std` offers no `From<i32> for f32` because the widening rounds.
#[allow(clippy::as_conversions, clippy::cast_precision_loss)]
pub const fn to_f32(value: i32) -> f32 {
    value as f32
}
