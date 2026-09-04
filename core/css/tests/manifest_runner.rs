//! Guards two-way consistency between `core/css/tests/data/MANIFEST.md` and the
//! crate's `SUPPORTED_PROPERTIES` / `SUPPORTED_SELECTORS` registries.
//!
//! The build fails if the code supports something the manifest omits, **or** the
//! manifest lists something the code does not support. B0 seeds the manifest
//! with the six computed properties `ComputedStyle` carries and no selectors;
//! the mechanism is real from the first slice and B5 (`core/html`) reuses it.
//!
//! `UPDATE_MANIFEST=1` rewrites the file from the registries (mirroring
//! `core/graphics/src/infrastructure/golden.rs`'s `UPDATE_GOLDEN`); the default
//! path compares and fails on divergence.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

const MANIFEST_VARIABLE: &str = "UPDATE_MANIFEST";

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("MANIFEST.md")
}

fn registry(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|entry| (*entry).to_owned()).collect()
}

/// The backtick-quoted names of list items under `## <section>`.
fn parse_section(markdown: &str, section: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut inside = false;
    for line in markdown.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            inside = heading.trim() == section;
            continue;
        }
        let Some(name) = list_item_name(line).filter(|_| inside) else {
            continue;
        };
        names.insert(name);
    }
    names
}

fn list_item_name(line: &str) -> Option<String> {
    line.trim()
        .strip_prefix("- ")?
        .strip_prefix('`')?
        .split('`')
        .next()
        .map(str::to_owned)
}

/// Every way `manifest` and `code` disagree, in both directions.
fn divergence(manifest: &BTreeSet<String>, code: &BTreeSet<String>) -> Vec<String> {
    let unlisted = code
        .difference(manifest)
        .map(|name| format!("code supports `{name}` but MANIFEST.md does not list it"));
    let unsupported = manifest
        .difference(code)
        .map(|name| format!("MANIFEST.md lists `{name}` but the code does not support it"));
    unlisted.chain(unsupported).collect()
}

fn render_manifest() -> String {
    let mut out = String::from(
        "# `core/css` support manifest\n\n\
         The CSS properties `core/css` resolves to a computed value, and the selector forms it matches against a `DomSnapshot`.\n\
         `core/css/tests/manifest_runner.rs` asserts this file and the crate's `SUPPORTED_PROPERTIES` / `SUPPORTED_SELECTORS`\n\
         registries agree **in both directions**: the build fails if the code supports something this file omits, or this file\n\
         lists something the code does not support. Regenerate with\n\
         `UPDATE_MANIFEST=1 cargo test -p css --test manifest_runner` (then run `pnpm format:md`).\n\n\
         ## Properties\n\n",
    );
    for property in css::SUPPORTED_PROPERTIES {
        let _ = writeln!(out, "- `{property}`");
    }
    out.push_str("\n## Selectors\n\n");
    push_selectors(&mut out);
    out
}

fn push_selectors(out: &mut String) {
    if css::SUPPORTED_SELECTORS.is_empty() {
        out.push_str("None yet — the selector engine arrives in B1.\n");
        return;
    }
    for selector in css::SUPPORTED_SELECTORS {
        let _ = writeln!(out, "- `{selector}`");
    }
}

fn read_or_bless() -> String {
    if std::env::var(MANIFEST_VARIABLE).is_ok_and(|value| value != "0") {
        let rendered = render_manifest();
        std::fs::write(manifest_path(), &rendered).expect("MANIFEST.md must be writable");
        println!(
            "{MANIFEST_VARIABLE} was set: wrote {}",
            manifest_path().display()
        );
        return rendered;
    }
    std::fs::read_to_string(manifest_path()).unwrap_or_else(|error| {
        panic!(
            "could not read {} ({error}). Run with {MANIFEST_VARIABLE}=1 to create it.",
            manifest_path().display()
        )
    })
}

#[test]
fn manifest_matches_the_support_registries() {
    let markdown = read_or_bless();
    let property_gap = divergence(
        &parse_section(&markdown, "Properties"),
        &registry(&css::SUPPORTED_PROPERTIES),
    );
    let selector_gap = divergence(
        &parse_section(&markdown, "Selectors"),
        &registry(&css::SUPPORTED_SELECTORS),
    );
    let gaps: Vec<String> = property_gap.into_iter().chain(selector_gap).collect();
    assert!(
        gaps.is_empty(),
        "MANIFEST.md and core/css disagree:\n{}",
        gaps.join("\n")
    );
}

#[test]
fn divergence_is_detected_in_both_directions() {
    let manifest = registry(&["display", "color"]);
    let code = registry(&["display", "margin"]);
    let gaps = divergence(&manifest, &code);

    assert!(
        gaps.iter()
            .any(|line| line.contains("code supports `margin`")),
        "an unlisted code capability must be reported"
    );
    assert!(
        gaps.iter()
            .any(|line| line.contains("MANIFEST.md lists `color`")),
        "an unsupported manifest entry must be reported"
    );
    assert!(
        divergence(&manifest, &manifest).is_empty(),
        "identical sets have no divergence"
    );
}
