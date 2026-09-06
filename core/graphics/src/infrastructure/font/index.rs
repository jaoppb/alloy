//! A lazily-built, process-wide index from a lowercased font-family name to the
//! file that provides it — what [`SystemFontProvider`] needs to resolve a
//! *named* family (`font-family: Georgia`), not just the three generics
//! [`FontCatalog::candidate_paths`] already covers.
//!
//! Best-effort, like the rest of this module: a machine with no font
//! directories is a valid empty index, never a panic, and no golden or
//! conformance test consults it.
//!
//! [`SystemFontProvider`]: crate::infrastructure::font::SystemFontProvider

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ttf_parser::Face;

use crate::infrastructure::font::catalog::FontCatalog;

/// OpenType `name` table id 16 — the typographic (preferred) family, used
/// ahead of id 1 when a face carries both.
const NAME_ID_TYPOGRAPHIC_FAMILY: u16 = 16;
/// OpenType `name` table id 1 — the legacy family name every face has.
const NAME_ID_FAMILY: u16 = 1;

/// The most font files the scan will parse before it stops — a bound so a
/// pathological font tree cannot stall the first render.
const MAX_FILES_SCANNED: usize = 4096;
/// How deep the scan recurses into subdirectories.
const MAX_DEPTH: usize = 8;

static INDEX: OnceLock<FontIndex> = OnceLock::new();

/// The shared index, built from the OS font directories on first use.
pub fn shared() -> &'static FontIndex {
    INDEX.get_or_init(FontIndex::build)
}

/// Lowercased family name → the first file that claimed it.
pub struct FontIndex {
    by_family: BTreeMap<String, PathBuf>,
}

impl FontIndex {
    fn build() -> Self {
        let mut by_family = BTreeMap::new();
        let mut scanned = 0_usize;
        for dir in FontCatalog::system_font_dirs() {
            scan_dir(&dir, MAX_DEPTH, &mut by_family, &mut scanned);
        }
        Self { by_family }
    }

    /// The file for `family`, matched case-insensitively, or `None` when no
    /// installed face claims that name.
    pub fn path_for(&self, family: &str) -> Option<&Path> {
        self.by_family
            .get(&family.to_ascii_lowercase())
            .map(PathBuf::as_path)
    }
}

fn scan_dir(dir: &Path, depth: usize, out: &mut BTreeMap<String, PathBuf>, scanned: &mut usize) {
    if depth == 0 || *scanned >= MAX_FILES_SCANNED {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, depth.saturating_sub(1), out, scanned);
            continue;
        }
        if !is_font_file(&path) {
            continue;
        }
        *scanned = scanned.saturating_add(1);
        index_file(&path, out);
        if *scanned >= MAX_FILES_SCANNED {
            return;
        }
    }
}

fn is_font_file(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    matches!(extension.as_deref(), Some("ttf" | "otf" | "ttc"))
}

fn index_file(path: &Path, out: &mut BTreeMap<String, PathBuf>) {
    let Ok(data) = std::fs::read(path) else {
        return;
    };
    let Ok(face) = Face::parse(&data, 0) else {
        return;
    };
    let Some(family) = family_name(&face) else {
        return;
    };
    out.entry(family.to_ascii_lowercase())
        .or_insert_with(|| path.to_path_buf());
}

fn family_name(face: &Face<'_>) -> Option<String> {
    named_string(face, NAME_ID_TYPOGRAPHIC_FAMILY).or_else(|| named_string(face, NAME_ID_FAMILY))
}

fn named_string(face: &Face<'_>, name_id: u16) -> Option<String> {
    face.names()
        .into_iter()
        .find(|entry| entry.name_id == name_id)
        .and_then(|entry| entry.to_string())
}
