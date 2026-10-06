# Full WHATWG named character references for `core/html` (issue #31)

## Requirements

Resolve every named character reference of WHATWG §13.5 — with and without the trailing `;` where the spec allows it —
and finish the numeric-reference rules, so that `html` decodes exactly what html5lib's reference cases expect, outputs
and parse errors included.

Boundaries: character references only; no table generator or `build.rs`; serialization changes only to the §13.3 escape
set. Done: all 4,290 vendored html5lib cases pass on tokens, codes and locations with no skip list, the manifest gate
and workspace gates are green.

## Entities

- `NamedCharacterReference(&'static ReferenceRow)` — handle on one row of the generated 2,125-name table; expansion is
  text (1–2 code points). `ESCAPED` = `AMP`, `LT`, `GT`, `QUOT`, `NBSP`, resolved in `const` (a missing name is a
  compile error). `ReferenceMatch` = a `longest_match` result (reference, characters consumed, terminated).
- `NumericReference(u32)` + `Radix` + `ResolvedReference` — §13.2.5.80: invalid → U+FFFD, C1 controls → Windows-1252,
  control/noncharacter reported.
- `ReferenceContext { Text, AttributeValue }` — the attribute-value literal rule of §13.2.5.73.
- Four `ParseErrorCode`s: `missing-semicolon-after-character-reference`, `control-character-reference`,
  `noncharacter-character-reference`, `unexpected-character-in-unquoted-attribute-value`.

## Approach

One generated table is the single source for decoding and for the serializer's escape set, which is a list of names
resolved at compile time. The tokenizer looks ahead on `Cursor::remaining()` instead of cloning the cursor, matches the
longest prefix, and reports every reference error at the `&` (ADR-0023 convention). html5lib locates the same errors at
the point of detection, so its runner maps them to the `&` through one documented rule.

## Structure

`domain/` holds the table, both reference types and the codes (zero dependencies, no I/O);
`infrastructure/tokenizer/ entity.rs` consumes them; `dom/application/serialize.rs` consumes
`NamedCharacterReference::from_char`. `html` stays a leaf crate; `serde_json` is a dev-dependency only.

## Operations

1. `domain/named_references.rs` — generated rows (source and sha256 in the header).
2. `domain/named_reference.rs`, `domain/numeric_reference.rs` — types, `const` lookup (`split_at_checked` + bytewise
   compare: `slice::get` and `<[u8]>::cmp` are not `const`).
3. `tokenizer/entity.rs` — `consume_character_reference(cursor, output, context)`; callers in `mod.rs` (Text) and
   `attribute_state.rs` (AttributeValue, now also unquoted values).
4. `dom` serializer — one `escape(raw, EscapeContext)`; `"` escaped only in attribute values.
5. Manifest rows + exact probes; `tests/html5lib_entities.rs` over `tests/data/html5lib/`.
6. `PORT_SCHEMA_VERSION` 3→4, PRD-008 row, contract record, CLAUDE.md.

## Norms

As in issue #36: a pure `domain/` (no I/O), no `unwrap`/`expect` on reachable paths, no `else`, one indentation level,
no boolean parameters, comments cite §13.2.5.x. `rumdl fmt` after editing Markdown.

## Safeguards

1. The table is verified independently: html5lib's cases were not used to generate it, and corrupting a row fails them.
2. A malformed reference never aborts a parse; unresolved ones stay literal text.
3. `dom` serialization changes only for characters outside `& < > " NBSP`; round-trips through the tokenizer.
4. Location rule is one function (`to_alloy_location`); no per-case exceptions, no skip list.
5. `html::PORT_SCHEMA_VERSION == 4`; `ParseErrorCode` stays `#[non_exhaustive]`.
