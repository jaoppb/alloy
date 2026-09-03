//! The golden-image gate: compare a rendered frame against a committed
//! reference, and make a failure diagnosable.
//!
//! ## What is compared, and why it matters
//!
//! The decoded [`Framebuffer`], never the PNG bytes. An encoder change — a
//! different block split, a future switch to real compression — would flip a
//! byte-comparison gate red while every pixel stayed identical. Comparing
//! pixels keeps the determinism gate measuring determinism (v0.3 report §2.5).
//!
//! ## Why a mismatch writes files
//!
//! "The golden image did not match" is not a diagnosis. On failure this writes
//! `<name>.actual.png` and `<name>.diff.png` next to the reference, so the
//! reviewer sees what was produced and exactly which pixels moved — which is the
//! difference between a five-minute fix and the days of bisection the v0.3
//! report warns about (risk §6.2).
//!
//! Like [`crate::conformance`], this is `pub` assertion code rather than
//! `#[cfg(test)]`, so `alloy` and every other crate can hold the same gate.

// Assertion code by nature: it panics on mismatch on purpose, and a missing
// reference is a test-authoring error, not a recoverable condition. Same
// carve-out, same reason, as `core/engine/src/conformance.rs:32-40`.
#![allow(clippy::panic, clippy::expect_used)]

use std::path::Path;

use crate::domain::color::Color;
use crate::domain::framebuffer::Framebuffer;
use crate::infrastructure::png;

/// The colour a difference map paints pixels that match.
const SAME: Color = Color::rgb(0, 0, 0);
/// The colour a difference map paints pixels that differ.
const DIFFERENT: Color = Color::rgb(255, 0, 0);

/// Asserts that `frame` matches the PNG committed at `reference`.
///
/// Set `UPDATE_GOLDEN=1` to write the reference instead of comparing — the
/// deliberate, explicit act of blessing a new expected image. It is an
/// environment variable rather than a flag so that it can never be the default,
/// and every run that uses it says so in its output.
///
/// # Panics
///
/// When the frames differ, when the reference is missing, or when it cannot be
/// decoded.
pub fn assert_matches_golden(frame: &Framebuffer, reference: &Path) {
    if blessing_enabled() {
        bless(frame, reference);
        return;
    }
    let expected = load(reference);
    if expected == *frame {
        return;
    }
    report_mismatch(frame, &expected, reference);
}

/// The environment variable that turns comparison into recording.
pub const UPDATE_VARIABLE: &str = "UPDATE_GOLDEN";

fn blessing_enabled() -> bool {
    std::env::var(UPDATE_VARIABLE).is_ok_and(|value| value != "0")
}

/// Writes `frame` as the new reference.
fn bless(frame: &Framebuffer, reference: &Path) {
    write(reference, &png::encode(frame));
    println!(
        "{UPDATE_VARIABLE} was set: wrote {} instead of comparing",
        reference.display()
    );
}

/// Reads and decodes the committed reference.
fn load(reference: &Path) -> Framebuffer {
    let bytes = std::fs::read(reference).unwrap_or_else(|error| {
        panic!(
            "golden reference {} could not be read ({error}). \
             Run with {UPDATE_VARIABLE}=1 to create it.",
            reference.display()
        )
    });
    png::decode(&bytes).unwrap_or_else(|problem| {
        panic!(
            "golden reference {} could not be decoded: {problem}",
            reference.display()
        )
    })
}

/// Writes the artefacts a reviewer needs, then fails with a counted summary.
fn report_mismatch(actual: &Framebuffer, expected: &Framebuffer, reference: &Path) {
    let actual_path = sibling(reference, "actual.png");
    write(&actual_path, &png::encode(actual));
    let summary = match difference_map(actual, expected) {
        Some((map, changed)) => {
            let diff_path = sibling(reference, "diff.png");
            write(&diff_path, &png::encode(&map));
            format!(
                "{changed} pixel(s) differ; wrote {} and {}",
                actual_path.display(),
                diff_path.display()
            )
        }
        None => format!(
            "the frames are {} and {} — different sizes; wrote {}",
            actual.size(),
            expected.size(),
            actual_path.display()
        ),
    };
    panic!("golden mismatch against {}: {summary}", reference.display());
}

/// A map painting every differing pixel red, plus how many differ.
///
/// `None` when the two frames are not the same size, in which case a per-pixel
/// map would be meaningless.
#[must_use]
pub fn difference_map(
    actual: &Framebuffer,
    expected: &Framebuffer,
) -> Option<(Framebuffer, usize)> {
    if actual.size() != expected.size() {
        return None;
    }
    let mut map = Framebuffer::filled(actual.size(), SAME)?;
    let mut changed = 0_usize;
    for row in 0..actual.height() {
        for column in 0..actual.width() {
            if actual.pixel(column, row) == expected.pixel(column, row) {
                continue;
            }
            changed = changed.saturating_add(1);
            map.set_pixel(column, row, DIFFERENT);
        }
    }
    Some((map, changed))
}

/// `reference` with its extension replaced by `suffix`.
fn sibling(reference: &Path, suffix: &str) -> std::path::PathBuf {
    reference.with_extension(suffix)
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!("could not create {}: {error}", parent.display());
        });
    }
    std::fs::write(path, bytes)
        .unwrap_or_else(|error| panic!("could not write {}: {error}", path.display()));
}
