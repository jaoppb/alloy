//! [`Scanner`] — the character cursor the tokenizer reads through.
//!
//! It exists because of the lint gate, not despite it: `string_slice` and
//! `indexing_slicing` are denied (`Cargo.toml:72,78`), so `&source[start..end]`
//! and `characters[index]` — the two moves every textbook tokenizer makes — are
//! not available. A `Vec<char>` read through `.get()` is, and it also gives the
//! three-character lookahead the grammar needs (`\` + hex, `/*`, `-->`, `+.5`)
//! where `Peekable<Chars>` gives one.
//!
//! Input is preprocessed exactly as CSS Syntax Level 3 §3.3 requires — every
//! newline form collapses to `\n` and `NULL` becomes `U+FFFD` — so the rest of
//! the tokenizer never has to think about `\r\n` or embedded nulls.

use crate::domain::error::SourceSpan;

/// The character CSS Syntax L3 §3.3 substitutes for `NULL`.
const REPLACEMENT_CHARACTER: &str = "\u{FFFD}";

/// A cursor over a preprocessed stylesheet, tracking 1-based line and column.
pub struct Scanner {
    characters: Vec<char>,
    position: usize,
    line: u32,
    column: u32,
}

impl Scanner {
    #[must_use]
    pub fn new(source: &str) -> Self {
        Self {
            characters: preprocess(source).chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// The character under the cursor.
    #[must_use]
    pub fn peek(&self) -> Option<char> {
        self.characters.get(self.position).copied()
    }

    /// The character `offset` places after the cursor.
    #[must_use]
    pub fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.position
            .checked_add(offset)
            .and_then(|index| self.characters.get(index))
            .copied()
    }

    /// Moves past the character under the cursor, keeping line and column
    /// current. A command: it answers nothing, so a caller that needs the
    /// character reads it with [`Scanner::peek`] first.
    pub fn consume(&mut self) {
        let Some(character) = self.peek() else {
            return;
        };
        self.position = self.position.saturating_add(1);
        self.record_position(character);
    }

    const fn record_position(&mut self, character: char) {
        if character == '\n' {
            self.line = self.line.saturating_add(1);
            self.column = 1;
            return;
        }
        self.column = self.column.saturating_add(1);
    }

    /// Where the cursor is now.
    #[must_use]
    pub const fn span(&self) -> SourceSpan {
        SourceSpan::new(self.line, self.column)
    }

    #[must_use]
    pub const fn is_exhausted(&self) -> bool {
        self.position >= self.characters.len()
    }
}

/// CSS Syntax L3 §3.3: normalise newlines and replace `NULL`.
fn preprocess(source: &str) -> String {
    source
        .replace("\r\n", "\n")
        .replace(['\r', '\u{000C}'], "\n")
        .replace('\0', REPLACEMENT_CHARACTER)
}
