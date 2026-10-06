# SPDD Analysis: Full WHATWG named character references in `html` (issue #31)

## Original Business Requirement

[jaoppb/alloy#31](https://github.com/jaoppb/alloy/issues/31) — "html: support the full WHATWG named character reference
table". `core/html/src/infrastructure/tokenizer/entity.rs::resolve_entity` resolves only numeric references plus a small
named subset; any other named reference (`&hellip;`, `&euro;`, `&larr;` …) is left unresolved. To do: cover the WHATWG
named character reference table (§13.5), including references that omit the trailing semicolon where the spec allows
it, and add conformance cases to `core/html/tests/data/MANIFEST.md` keeping the two-way gate in `manifest_runner.rs`
green. Raised in review of #17.

## Current State (as found)

- The issue text is stale: `HtmlEntity` (`core/html/src/domain/entity.rs`) already has ~46 variants, not 10.
- `HtmlEntity` does two jobs: the tokenizer decodes with `from_name` (`entity.rs:115`), and the `dom` serializer
  encodes with `from_char` (`core/dom/src/application/serialize.rs:126`/`:137`).
- It cannot hold the full table: some references expand to two code points (`&NotEqualTilde;`), and one character has
  many names (`&amp;`/`&AMP;`), so `from_char` stops being 1:1.
- The tokenizer caps a candidate at 16 characters (`entity.rs:35`); the longest name is 33. It has no semicolon-less
  matching, no longest-prefix match and no attribute-context rule.
- Numeric references are incomplete: no 0x80–0x9F Windows-1252 remap, no `control-character-reference` or
  `noncharacter-character-reference`, and `&#12ab;` is explicitly left out (`entity.rs:82`).
- The serializer today also writes `&copy;`, `&euro;` … which WHATWG §13.3 does not (an unrecorded deviation).

## Decisions

1. **One table, one type.** `HtmlEntity` becomes a handle into a single generated table (`struct HtmlEntity(u16)`, a
   row index). `from_name` searches all ~2,231 rows; `from_char` searches only the escape set; `as_char()` is replaced
   by `expansion() -> &'static str` (1–2 code points); `as_entity()` / `entity_name()` stay. The enum variants go, so
   `html::PORT_SCHEMA_VERSION` 3→4 and a PRD-008 migration row.
2. **The escape set is a hand-written list of names** (`ESCAPED`), resolved against the table in a `const` block. A typo
   or missing name is a compile error, not a test. No characters are duplicated between the table and the set.
3. **The escape set shrinks to WHATWG §13.3**: `amp`, `lt`, `gt`, `quot`, `nbsp`. `dom` keeps the text-vs-attribute
   split. `dom` output changes (`&copy;`, `&euro;`, `&apos;` … are no longer written), so `core/dom/tests/serialize.rs`
   (line 110) is rewritten.
4. **The table is generated once and committed** from `https://html.spec.whatwg.org/entities.json` (frozen upstream).
   The header records the URL and sha256, rows are sorted by name; no kept generator, no `build.rs`, no new runtime
   dependency.
5. **Matching follows §13.2.5.73**: longest-prefix match over the table; legacy names without `;` resolve and report a
   new `missing-semicolon-after-character-reference`; in an attribute value an unterminated match followed by `=` or an
   alphanumeric stays literal. The 16-character cap goes.
6. **Numeric references are in scope** (§13.2.5.80 completed): the Windows-1252 remap, the new
   `control-character-reference` and `noncharacter-character-reference` codes, and `&#12ab;` per spec.
7. **Oracle: vendored html5lib-tests** `tokenizer/namedEntities.test` and `entities.test` (upstream sha recorded) under
   `core/html/tests/data/html5lib/`, run by a new integration test with `serde_json` as a **dev-dependency** (already in
   `Cargo.lock`; `unsafe-audit` scans direct normal dependencies only). It asserts decoded output, diagnostic codes and
   line/column.
8. **Manifest**: new syntax rows (legacy no-semicolon, multi-code-point, attribute rule) and parse-error rows for the
   three new codes, with probes in `manifest_runner.rs` and `SUPPORTED_SYNTAX` updated, so the two-way gate stays green.

## Non-goals

- Changing anything else in serialization beyond the §13.3 escape set (text vs attribute rules stay in `dom`).
- A table generator, `build.rs`, or any runtime dependency.
- Tokenizer states unrelated to character references.

## Open questions

- **Name of the type.** Keep `HtmlEntity` (smallest diff, schema bump already paid) or rename to
  `NamedCharacterReference` (matches §13.5, but churns every import).
- **Location convention.** html5lib reports line/col by its own convention; if `Cursor::last_location` differs, either
  adapt the runner's mapping or change `Cursor`. Decide once the first failing case shows the real delta.

## Risks

- A location off-by-one forces `Cursor` changes that touch every diagnostic.
- The html5lib fixture is ~1 MB of vendored data.
- The `const` binary search over `str` must be byte-wise and hand-written; subtle.
- The one-time-generated table is verified only by the oracle, not reproducibly.
- The serialization change is visible to `alloy` users (`alloy/src/main.rs`) and to scripts using `serialize_html`.
