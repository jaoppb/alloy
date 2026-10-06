# SPDD Analysis: Full WHATWG named character references in `html` (issue #31)

## Original Business Requirement

[jaoppb/alloy#31](https://github.com/jaoppb/alloy/issues/31) — "html: support the full WHATWG named character reference
table". `core/html/src/infrastructure/tokenizer/entity.rs::resolve_entity` resolves only numeric references plus a small
named subset; any other named reference (`&hellip;`, `&euro;`, `&larr;` …) is left unresolved. To do: cover the WHATWG
named character reference table (§13.5), including references that omit the trailing semicolon where the spec allows it,
and add conformance cases to `core/html/tests/data/MANIFEST.md` keeping the two-way gate in `manifest_runner.rs` green.
Raised in review of #17.

## Current State (as found)

- The issue text is stale: `HtmlEntity` (`core/html/src/domain/entity.rs`) already has ~46 variants, not 10.
- `HtmlEntity` does two jobs: the tokenizer decodes with `from_name` (`entity.rs:115`), and the `dom` serializer encodes
  with `from_char` (`core/dom/src/application/serialize.rs:126`/`:137`).
- It cannot hold the full table: some references expand to two code points (`&NotEqualTilde;`), and one character has
  many names (`&amp;`/`&AMP;`), so `from_char` stops being 1:1.
- The tokenizer caps a candidate at 16 characters (`entity.rs:35`); the longest name is 33. It has no semicolon-less
  matching, no longest-prefix match and no attribute-context rule.
- Numeric references are incomplete: no 0x80–0x9F Windows-1252 remap, no `control-character-reference` or
  `noncharacter-character-reference`, and `&#12ab;` is explicitly left out (`entity.rs:82`).
- The serializer today also writes `&copy;`, `&euro;` … which WHATWG §13.3 does not (an unrecorded deviation).

## Decisions

1. **One table, one type, renamed.** `HtmlEntity` is replaced by `NamedCharacterReference`, a handle into a single
   generated table (`struct NamedCharacterReference(&'static ReferenceRow)`; WHATWG §13.5 term). Every import, ADR-0024,
   PRD-008 row 3 and the `CLAUDE.md` vocabulary list are updated in the same PR. `from_name` searches all 2,125 names
   (2,231 entries; 106 legacy names are one row each with a flag); `from_char` searches only the escape set; `as_char()`
   is replaced by `expansion() -> &'static str` (1–2 code points); `as_entity()` / `entity_name()` stay (the `dom`
   serializer calls them on the new type). The enum variants go, so `html::PORT_SCHEMA_VERSION` 3→4 and a PRD-008
   migration row.
2. **The escape set is a hand-written list of names** (`ESCAPED`), resolved against the table in a `const` block. A typo
   or missing name is a compile error, not a test. No characters are duplicated between the table and the set.
3. **The escape set shrinks to WHATWG §13.3**: `amp`, `lt`, `gt`, `quot`, `nbsp`. `dom` keeps the text-vs-attribute
   split. `dom` output changes (`&copy;`, `&euro;`, `&apos;` … are no longer written, nor `&quot;` in text), so
   `core/dom/tests/serialize.rs` is rewritten. Attribute values keep escaping `<` and `>` (behaviour unchanged, the safe
   side).
4. **The table is generated once and committed.** `html.spec.whatwg.org` was unreachable from the sandbox, so the source
   is CPython 3.11's `html.entities.html5` (a verbatim mirror of `entities.json`); the header records that and a sha256
   over the canonical rows, which are sorted by name; no kept generator, no `build.rs`, no new runtime dependency.
5. **Matching follows §13.2.5.73**: longest-prefix match over the table; legacy names without `;` resolve and report a
   new `missing-semicolon-after-character-reference`; in an attribute value an unterminated match followed by `=` or an
   alphanumeric stays literal. The 16-character cap goes.
6. **Numeric references are in scope** (§13.2.5.80 completed): the Windows-1252 remap, the new
   `control-character-reference` and `noncharacter-character-reference` codes, and `&#12ab;` per spec. `&` is now also
   decoded in unquoted attribute values, which reports the fourth new code,
   `unexpected-character-in-unquoted-attribute-value` (html5lib's `entities.test` expects it).
7. **Oracle: vendored html5lib-tests** `tokenizer/namedEntities.test` and `entities.test` (upstream sha recorded) under
   `core/html/tests/data/html5lib/`, run by a new integration test with `serde_json` as a **dev-dependency** (already in
   `Cargo.lock`; `unsafe-audit` scans direct normal dependencies only). It asserts decoded output, diagnostic codes and
   line/column.
8. **Location rule.** Our convention (ADR-0023, #36) is the contract: character-reference errors are located at the `&`.
   html5lib locates them where the spec detects them (after `&name`, at the `;`, after the `;`), so the runner maps
   those codes to the closest `&` before html5lib's position (`to_alloy_location`, one rule, no per-case fixes); every
   other code compares as is. `Cursor` is not changed.
9. **Done = zero skipped cases.** Every case in `namedEntities.test` and `entities.test` passes on output, codes and
   location, with no skip list; a deviation is fixed in the tokenizer or blocks the PR.
10. **Manifest**: new syntax rows (legacy no-semicolon, multi-code-point, attribute rule) and parse-error rows for the
   three new codes, with probes in `manifest_runner.rs` and `SUPPORTED_SYNTAX` updated, so the two-way gate stays green.

## Done (v1 of this change)

- `domain/named_references.rs` (generated, 2,125 rows), `domain/named_reference.rs`, `domain/numeric_reference.rs`;
  tokenizer `entity.rs` rewritten over them; four new parse-error codes with manifest rows and exact probes;
  `html::PORT_SCHEMA_VERSION` 3→4 with the PRD-008 migration row.
- `core/html/tests/html5lib_entities.rs` replays 4,290 html5lib cases (tokens, codes, locations), no skips. Corrupting a
  table row or a legacy flag makes it fail.

## Non-goals

- Changing anything else in serialization beyond the §13.3 escape set (text vs attribute rules stay in `dom`).
- A table generator, `build.rs`, or any runtime dependency.
- Tokenizer states unrelated to character references.

## Open questions

None. Remaining choices (e.g. the context enum replacing a boolean flag on `consume_character_reference`, vendored file
licence header) are implementation details.

## Risks

- The location mapping can hide a real mislocation; it must stay one documented rule, never per-case fixes.
- The html5lib fixture is ~1 MB of vendored data.
- The `const` binary search over `str` must be byte-wise and hand-written; subtle.
- The one-time-generated table is verified only by the oracle, not reproducibly.
- The serialization change is visible to `alloy` users (`alloy/src/main.rs`) and to scripts using `serialize_html`.
