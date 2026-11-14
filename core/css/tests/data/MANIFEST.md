# `core/css` support manifest

The declared cut of `docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md` §2.8, property by property and selector by selector —
plus the handful of tokens brought forward from the v0.7 CSS widening, marked `ddg` in the `since` column and recorded
in `docs/requirements/PRD-007-*.md` §6. Everything **not** listed here is refused by the parser and recorded as a
`ParseNote` — never accepted and silently ignored, which is the shrinkage this file exists to prevent (§2.8:350-354).

`core/css/tests/manifest_runner.rs` checks three things and fails loudly on any of them:

1. this file and `css::SUPPORTED_PROPERTIES` / `css::SUPPORTED_SELECTORS` name the same sets, **in both directions**;
2. every listed token has a probe that the parser really accepts, and — for a property — that really changes the
   computed style the cascade produces;
3. a battery of forms declared **out** is really refused, each with a note.

There is **no bless path**: this file is hand-maintained, because the `notes` column carries reasons no generator can
invent. A new supported token means editing the registry in `core/css/src/lib.rs`, this table, and the probe table in
the runner — all three, or CI is red.

`since` is the phase of `~/.claude/plans/…-fancy-dijkstra.md` that landed the token; `ddg` marks a token brought forward
from v0.7 for the "unstyled real sites" follow-up (`docs/reports/DIAGNOSTICO-JANELA-BRANCA-WAYLAND.md` §3).

## Properties

The CSS properties the parser accepts inside a declaration block and the cascade resolves to a computed value. A
declaration naming anything else is dropped on its own, with a note, leaving the rest of its rule intact.

| token                        | since | notes                                                                                                                                                                                      |
| ---------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `display`                    | B1    | `none` / `block` / `inline` / `flex` (B4) / `inline-block` (atomic; shrink-to-fit by measuring layout; baseline = bottom margin edge) / `list-item` (as `block`, no marker)                |
| `color`                      | B1    | inherited; `#rgb`, `#rrggbb`, the 17 basic colour names, and `rgb()` / `rgba()` (B2)                                                                                                       |
| `background-color`           | B1    | not inherited; same value grammar as `color`                                                                                                                                               |
| `background`                 | ddg   | shorthand narrowed to the background **colour**; `url()`, gradients, position/size/repeat scanned past, `none` clears; the image is not fetched (v0.7)                                     |
| `margin`                     | B1    | the 1–4 component shorthand (CSS Box Model §8.3)                                                                                                                                           |
| `margin-top`                 | B1    | longhand; overwrites only its own side                                                                                                                                                     |
| `margin-right`               | B1    | longhand                                                                                                                                                                                   |
| `margin-bottom`              | B1    | longhand                                                                                                                                                                                   |
| `margin-left`                | B1    | longhand                                                                                                                                                                                   |
| `padding`                    | B1    | the 1–4 component shorthand                                                                                                                                                                |
| `padding-top`                | B1    | longhand                                                                                                                                                                                   |
| `padding-right`              | B1    | longhand                                                                                                                                                                                   |
| `padding-bottom`             | B1    | longhand                                                                                                                                                                                   |
| `padding-left`               | B1    | longhand                                                                                                                                                                                   |
| `font-size`                  | B1    | inherited; `px` / `em` / `rem` / `%` / `pt`, and the unitless `0`                                                                                                                          |
| `font-family`                | fonts | inherited; comma list of quoted / bare names and `serif` / `sans-serif` / `monospace` — see the font simplifications below                                                                 |
| `border-width`               | B4    | the 1–4 component shorthand; `border-style` / `border-color` stay out of the cut                                                                                                           |
| `border`                     | ddg   | shorthand setting width, style and colour; omitted style defaults to `none`, omitted colour to `currentColor` (see border simplifications)                                                  |
| `width`                      | B4    | `Sizing::Auto` or a `Length`; the flow content width when auto                                                                                                                             |
| `height`                     | B4    | `Sizing::Auto` or a `Length`; a `%` against an indefinite container computes to auto                                                                                                       |
| `box-sizing`                 | B4    | `content-box` (initial) / `border-box` (CSS Box Sizing L3 §5)                                                                                                                              |
| `text-align`                 | B4    | inherited; `left` (initial) / `right` / `center` / `justify`                                                                                                                               |
| `white-space`                | B4    | not inherited; `normal` (initial) / `pre` / `nowrap`                                                                                                                                       |
| `border-top-width`           | B4    | longhand; overwrites only its own side                                                                                                                                                     |
| `border-right-width`         | B4    | longhand                                                                                                                                                                                   |
| `border-bottom-width`        | B4    | longhand                                                                                                                                                                                   |
| `border-left-width`          | B4    | longhand                                                                                                                                                                                   |
| `flex-direction`             | B4    | `row` (initial) / `row-reverse` / `column` / `column-reverse`                                                                                                                              |
| `flex-wrap`                  | B4    | `nowrap` (initial) / `wrap` / `wrap-reverse` — see the Flexbox simplifications below                                                                                                       |
| `justify-content`            | B4    | `flex-start` (initial) / `flex-end` / `center` / `space-between` / `space-around` / `space-evenly`                                                                                         |
| `align-items`                | B4    | `stretch` (initial) / `flex-start` / `flex-end` / `center` / `baseline`                                                                                                                    |
| `align-content`              | B4    | `stretch` (initial) / `flex-start` / `flex-end` / `center` / `space-between` / `space-around`                                                                                              |
| `align-self`                 | B4    | `auto` (initial, defers to `align-items`) / same keywords as `align-items`                                                                                                                 |
| `flex-grow`                  | B4    | a non-negative number; initial `0`                                                                                                                                                         |
| `flex-shrink`                | B4    | a non-negative number; initial `1`                                                                                                                                                         |
| `flex-basis`                 | B4    | `auto` (initial) or a `Sizing` length/percentage                                                                                                                                           |
| `position`                   | P1    | `static` (initial) / `relative` / `absolute` / `fixed` / `sticky` (CSS Positioned Layout L3 §2)                                                                                            |
| `top`                        | P1    | `auto` (initial) or a `Sizing` length/percentage                                                                                                                                           |
| `right`                      | P1    | `auto` (initial) or a `Sizing` length/percentage                                                                                                                                           |
| `bottom`                     | P1    | `auto` (initial) or a `Sizing` length/percentage                                                                                                                                           |
| `left`                       | P1    | `auto` (initial) or a `Sizing` length/percentage                                                                                                                                           |
| `z-index`                    | P1    | `auto` (initial) or an integer level                                                                                                                                                       |
| `min-width`                  | P1    | `auto` (initial) or a `Sizing` length/percentage                                                                                                                                           |
| `max-width`                  | P1    | `auto`/`none` (initial) or a `Sizing` length/percentage                                                                                                                                    |
| `min-height`                 | P1    | `auto` (initial) or a `Sizing` length/percentage                                                                                                                                           |
| `max-height`                 | P1    | `auto`/`none` (initial) or a `Sizing` length/percentage                                                                                                                                    |
| `overflow`                   | P1    | 1–2 component shorthand (`visible`, `hidden`, `clip`, `scroll`, `auto`)                                                                                                                    |
| `overflow-x`                 | P1    | `visible` (initial) / `hidden` / `clip` / `scroll` / `auto`                                                                                                                                |
| `overflow-y`                 | P1    | `visible` (initial) / `hidden` / `clip` / `scroll` / `auto`                                                                                                                                |
| `border-style`               | P2    | 1–4 value shorthand (`none`, `solid`, `dashed`, `dotted`, `double`, etc.)                                                                                                                  |
| `border-top-style`           | P2    | longhand                                                                                                                                                                                   |
| `border-right-style`         | P2    | longhand                                                                                                                                                                                   |
| `border-bottom-style`        | P2    | longhand                                                                                                                                                                                   |
| `border-left-style`          | P2    | longhand                                                                                                                                                                                   |
| `border-color`               | P2    | 1–4 value shorthand (`#rgb`, `#rrggbb`, named colours)                                                                                                                                     |
| `border-top-color`           | P2    | longhand                                                                                                                                                                                   |
| `border-right-color`         | P2    | longhand                                                                                                                                                                                   |
| `border-bottom-color`        | P2    | longhand                                                                                                                                                                                   |
| `border-left-color`          | P2    | longhand                                                                                                                                                                                   |
| `border-radius`              | P2    | 1–4 corner shorthand                                                                                                                                                                       |
| `border-top-left-radius`     | P2    | longhand corner radius                                                                                                                                                                     |
| `border-top-right-radius`    | P2    | longhand corner radius                                                                                                                                                                     |
| `border-bottom-right-radius` | P2    | longhand corner radius                                                                                                                                                                     |
| `border-bottom-left-radius`  | P2    | longhand corner radius                                                                                                                                                                     |
| `box-shadow`                 | P2    | offsets, blur, spread, colour, inset                                                                                                                                                       |
| `opacity`                    | P2    | clamped `0.0`..`1.0`                                                                                                                                                                       |
| `background-image`           | P2    | `none` or `url(...)`                                                                                                                                                                       |
| `background-position`        | P2    | keywords, lengths, percentages                                                                                                                                                             |
| `background-size`            | P2    | `auto`, `cover`, `contain`, explicit                                                                                                                                                       |
| `background-repeat`          | P2    | `repeat`, `no-repeat`, `repeat-x`, `repeat-y`                                                                                                                                              |
| `font-weight`                | P2    | `normal` (400), `bold` (700), numeric weights, `bolder`, `lighter`                                                                                                                         |
| `font-style`                 | P2    | `normal`, `italic`, `oblique`                                                                                                                                                              |
| `line-height`                | P2    | `normal`, lengths, unitless factor, percentages                                                                                                                                            |
| `letter-spacing`             | P2    | `normal` or lengths                                                                                                                                                                        |
| `word-spacing`               | P2    | `normal` or lengths                                                                                                                                                                        |
| `text-decoration`            | P2    | shorthand combining line, style, and colour                                                                                                                                                |
| `text-decoration-line`       | P2    | `none`, `underline`, `overline`, `line-through`                                                                                                                                            |
| `text-decoration-color`      | P2    | decoration colour                                                                                                                                                                          |
| `text-decoration-style`      | P2    | `solid`, `double`, `dotted`, `dashed`, `wavy`                                                                                                                                              |
| `text-transform`             | P2    | `none`, `capitalize`, `uppercase`, `lowercase`                                                                                                                                             |
| `text-overflow`              | P2    | `clip`, `ellipsis`                                                                                                                                                                         |
| `overflow-wrap`              | P2    | `normal`, `break-word`, `anywhere`                                                                                                                                                         |
| `word-wrap`                  | P2    | legacy alias of `overflow-wrap`                                                                                                                                                            |
| `word-break`                 | P2    | `normal`, `break-all`, `keep-all`                                                                                                                                                          |
| `grid-template-columns`      | P2    | track sizing (`px`, `%`, `fr`, `auto`, `minmax()`, `repeat()`)                                                                                                                             |
| `grid-template-rows`         | P2    | track sizing                                                                                                                                                                               |
| `grid-template-areas`        | P2    | named area rectangles                                                                                                                                                                      |
| `grid-auto-columns`          | P2    | implicit track sizing                                                                                                                                                                      |
| `grid-auto-rows`             | P2    | implicit track sizing                                                                                                                                                                      |
| `grid-auto-flow`             | P2    | `row`, `column`, `dense`, etc.                                                                                                                                                             |
| `grid-column`                | P2    | line placement shorthand                                                                                                                                                                   |
| `grid-column-start`          | P2    | line index or span                                                                                                                                                                         |
| `grid-column-end`            | P2    | line index or span                                                                                                                                                                         |
| `grid-row`                   | P2    | line placement shorthand                                                                                                                                                                   |
| `grid-row-start`             | P2    | line index or span                                                                                                                                                                         |
| `grid-row-end`               | P2    | line index or span                                                                                                                                                                         |
| `grid-area`                  | P2    | area placement shorthand                                                                                                                                                                   |
| `gap`                        | P2    | row and column gutters shorthand                                                                                                                                                           |
| `row-gap`                    | P2    | row gutter                                                                                                                                                                                 |
| `column-gap`                 | P2    | column gutter                                                                                                                                                                              |
| `grid-gap`                   | P2    | legacy alias of `gap`                                                                                                                                                                      |
| `grid-row-gap`               | P2    | legacy alias of `row-gap`                                                                                                                                                                  |
| `grid-column-gap`            | P2    | legacy alias of `column-gap`                                                                                                                                                               |
| `writing-mode`               | P2    | `horizontal-tb`, `vertical-rl`, `vertical-lr`                                                                                                                                              |
| `direction`                  | P2    | `ltr`, `rtl`                                                                                                                                                                               |
| `inline-size`                | P2    | logical sizing mapped to physical axis                                                                                                                                                     |
| `block-size`                 | P2    | logical sizing mapped to physical axis                                                                                                                                                     |
| `min-inline-size`            | P2    | logical min constraint                                                                                                                                                                     |
| `min-block-size`             | P2    | logical min constraint                                                                                                                                                                     |
| `max-inline-size`            | P2    | logical max constraint                                                                                                                                                                     |
| `max-block-size`             | P2    | logical max constraint                                                                                                                                                                     |
| `margin-inline`              | P2    | logical margin shorthand                                                                                                                                                                   |
| `margin-inline-start`        | P2    | logical margin                                                                                                                                                                             |
| `margin-inline-end`          | P2    | logical margin                                                                                                                                                                             |
| `margin-block`               | P2    | logical margin shorthand                                                                                                                                                                   |
| `margin-block-start`         | P2    | logical margin                                                                                                                                                                             |
| `margin-block-end`           | P2    | logical margin                                                                                                                                                                             |
| `padding-inline`             | P2    | logical padding shorthand                                                                                                                                                                  |
| `padding-inline-start`       | P2    | logical padding                                                                                                                                                                            |
| `padding-inline-end`         | P2    | logical padding                                                                                                                                                                            |
| `padding-block`              | P2    | logical padding shorthand                                                                                                                                                                  |
| `padding-block-start`        | P2    | logical padding                                                                                                                                                                            |
| `padding-block-end`          | P2    | logical padding                                                                                                                                                                            |
| `border-inline`              | P2    | logical border shorthand                                                                                                                                                                   |
| `border-inline-width`        | P2    | logical border width shorthand                                                                                                                                                             |
| `border-inline-start-width`  | P2    | logical border width                                                                                                                                                                       |
| `border-inline-end-width`    | P2    | logical border width                                                                                                                                                                       |
| `border-block`               | P2    | logical border shorthand                                                                                                                                                                   |
| `border-block-width`         | P2    | logical border width shorthand                                                                                                                                                             |
| `border-block-start-width`   | P2    | logical border width                                                                                                                                                                       |
| `border-block-end-width`     | P2    | logical border width                                                                                                                                                                       |
| `inset-inline`               | P2    | logical insets shorthand                                                                                                                                                                   |
| `inset-inline-start`         | P2    | logical inset                                                                                                                                                                              |
| `inset-inline-end`           | P2    | logical inset                                                                                                                                                                              |
| `inset-block`                | P2    | logical insets shorthand                                                                                                                                                                   |
| `inset-block-start`          | P2    | logical inset                                                                                                                                                                              |
| `inset-block-end`            | P2    | logical inset                                                                                                                                                                              |
| `visibility`                 | ddg   | inherited; `visible` (initial) / `hidden` / `collapse` (treated as `hidden` outside tables; CSS 2.2 §11.2)                                                                                |
| `outline`                    | ddg   | shorthand; `outline-color`, `outline-style`, `outline-width` (CSS UI 4 §3)                                                                                                                 |
| `outline-offset`             | ddg   | non-inherited; `<length>` distance from border edge (CSS UI 4 §3.4)                                                                                                                        |
| `outline-color`              | ddg   | non-inherited; `<color>` (CSS UI 4 §3.1)                                                                                                                                                   |
| `outline-width`              | ddg   | non-inherited; `<length>` or `thin` / `medium` / `thick` (CSS UI 4 §3.2)                                                                                                                   |
| `outline-style`              | ddg   | non-inherited; `none` (initial) / `auto` / `solid` / etc. (CSS UI 4 §3.3)                                                                                                                 |

Every property above also accepts the CSS-wide keywords `initial` and `inherit` (B2, CSS Cascade L4 §7.1): `initial`
resets to the value `ComputedStyle::initial()` gives it, `inherit` copies the parent's computed value even for a
property that does not normally inherit. `unset` and `revert` are not recognised.

Declared **out** for v0.5, and refused with a note: `float`, `clear`, and every other property. (`background` and
`border` are now **in**, narrowed to one component each — see the rows above and `PORT_SCHEMA_VERSION = 5`; the
`background`/`border` _longhands_ other than `-width` stay out.) `!important` is parsed, preserved on the declaration,
**and** wins the cascade as of B2 (`plano:435-443`): CSS Cascade L4 §4.2's origin/importance ordering, `User` origin
included by construction even though nothing sources it yet.

### Flexbox simplifications (B4)

The Flexbox algorithm (`core/css/src/infrastructure/layout/flex.rs`) is deliberately not the full CSS Flexbox L1 §9
algorithm. Four cuts, each pre-approved by the v0.5 B4 handoff as an explicit relief valve rather than a silent
shrinkage:

| gap                                                                                                    | behaviour instead                                                                                                                                                                  | tracked for |
| ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| multi-line wrapping needs an intrinsic-sizing pass to size a `column` container with an auto main size | `flex-wrap: wrap` / `wrap-reverse` only break lines when the main axis has a **definite** size to overflow against; an auto-sized `column` container never wraps                   | v0.7        |
| no first-baseline-of-line algorithm                                                                    | `align-items: baseline` / `align-self: baseline` behave like `flex-start`                                                                                                          | v0.7        |
| no shrink-to-fit (min/max-content) intrinsic sizing pass                                               | an item with both `flex-basis: auto` and an `auto` own main-size property gets a hypothetical main size of `0` instead of a content-based measurement                              | v0.7        |
| same missing shrink-to-fit pass, on the cross axis                                                     | in a `column` container, a non-`stretch` item with an `auto` own width still fills the cross axis exactly as `stretch` would; an item with an explicit `width` positions correctly | v0.7        |

### Inline formatting simplifications

The inline formatting context (`core/css/src/infrastructure/layout/inline.rs`) gives an inline box or a text node one
fragment per line it touches (#134); a text fragment carries the text on its line, which is what paint draws. A
block-level box inside an inline box splits the run around it (CSS 2.1 §9.2.1.1, #135).

| gap                                                                                                                                                                   | behaviour instead                                                                                                                                                                | tracked for |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| an inline box's own `margin`, `border` and `padding` (CSS 2.1 §10.3.1, §10.6.1)                                                                                       | an inline box's fragments are the bounds of its content; its edges are not laid out                                                                                              | v0.7        |
| `text-align: justify` spacing in paint                                                                                                                                | layout widens a justified line's gaps, but a text fragment carries its text at natural spacing, so paint draws the words bunched at the line start inside the justified fragment | v0.7        |
| a box that lays its own text out (an `<input>` / `<select>` label, a flex-blockified text node) also owns that text's line fragments                                  | its background and outline are painted again around the label line, inside the control                                                                                           | v0.7        |
| shrink-to-fit width of an `inline-block` whose content holds a block-level box — directly, or through a block-in-inline split such as `<a><div>card</div></a>` (#135) | the block is measured at the full available width, so the `inline-block` fills it instead of shrinking to its content (no min/max-content pass)                                  | v0.7        |

### Font simplifications (fonts increment)

`font-family` is an inline field of `ComputedStyle` (`core/css/src/domain/computed/font.rs`), which is cloned at every
cascade step and stored per styled node, so the list is fixed-capacity rather than a `Vec` that would allocate on each
clone. (`ComputedStyle` itself is `Clone`, not `Copy`, since `css::PORT_SCHEMA_VERSION` `8`.) Two cuts, silent (the
value is parsed only at cascade time, which has no `ParseNote` channel) and declared here in the same spirit as the
Flexbox cuts:

| gap                                      | behaviour instead                                                                                                                                                 | tracked for |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| the list has no length limit in CSS      | at most `FontFamilyList::CAPACITY` (3) families are kept; a real chain ends in a generic and the provider default is a generic, so the dropped tail is equivalent | v0.7        |
| a family name has no length limit in CSS | a name longer than `FamilyName::CAPACITY` (23) bytes is truncated at a UTF-8 boundary                                                                             | v0.7        |

### Grid storage caps

The grid computed values (`core/css/src/domain/computed/grid/`) are `Copy` and fixed-capacity, so a `GridStyle` is one
flat value; `ComputedStyle` keeps it behind a shared `Arc` that stays empty while every grid property is `initial`, so
the caps cost nothing on a node that sets no grid property. Unlike the font cuts, exceeding a cap
**refuses the whole declaration** (the previous value stands) and the cascade parser emits a `tracing::warn!` — a
truncated track list or area map would lay out a different grid, which is worse than no grid. Names are case-sensitive
`<custom-ident>`s (CSS Values 4 §4.2): `Nav` and `nav` are two areas.

| gap                                                       | behaviour instead                                                                              | tracked for |
| --------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | ----------- |
| a track list has no length limit in CSS                   | more than `TrackList::CAPACITY` (16) tracks, after `repeat()` expansion, is refused            | v0.7        |
| `grid-template-areas` has no cell limit in CSS            | more than `GridTemplateAreas::CAPACITY` (64) cells is refused                                  | v0.7        |
| area and line names have no length limit in CSS           | a name longer than `GridAreaName::CAPACITY` / `GridLineName::CAPACITY` (32) bytes is refused   | v0.7        |
| `repeat()` takes `auto-fill` / `auto-fit` and named lines | only an integer count and plain track sizes; a nested `repeat()` is invalid CSS and is refused | v0.7        |

`box-shadow` follows the same rule: more than `BoxShadowList::CAPACITY` (4) shadows is refused with a warning, never
truncated.

### Border simplifications

The `border-style` keywords `dashed`, `dotted`, `double`, `groove`, `ridge`, `inset`, and `outset` are parsed and stored
in the domain model (`core/css/src/domain/computed/visual/border_style.rs`), but are rendered as solid rectangles by the
software rasterizer (`alloy/src/application/paint.rs`).

| gap                                                               | behaviour instead                   | tracked for |
| ----------------------------------------------------------------- | ----------------------------------- | ----------- |
| stroke pattern rasterization (`dashed`, `dotted`, `double`, etc.) | rendered as solid filled rectangles | v0.7        |

### Custom properties

Custom properties (`--*`, CSS Variables L1) are the one deliberate exception to "everything not listed is refused": the
set of names is open by definition, so the parser accepts any valid `--name` (case preserved — `--C` and `--c` are two
properties) **without** an entry in the table above or in `css::SUPPORTED_PROPERTIES`, and `manifest_runner.rs` has no
probe for them. They are cascaded per element outside `ComputedStyle` and feed `var()` in any listed property; a `var()`
that cannot be substituted (undefined without a fallback, a cycle, more than 64 KiB of output) makes its declaration
invalid at computed-value time, i.e. `unset`. The full rules are in `docs/architecture/style-cascade-port-contract.md`
("Cascade semantics of schema 7"); the regression tests are `core/css/tests/cascade_semantics.rs`.

### Form controls and fieldset/legend

Form controls receive UA default styles according to WHATWG HTML §15.5 and §15.3.12 (issue #87).

| gap                                                | behaviour instead                                                                        | tracked for |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------- | ----------- |
| `<fieldset>` / `<legend>` border-notch clipping    | `<fieldset>` renders standard rectangular borders; `<legend>` is rendered in normal flow | v0.7        |

### UA sheet rules the cascade cannot express yet

`core/css/assets/ua.css` follows WHATWG HTML §15.3 section by section (#133, part of #81). The rules below are in
§15.3.1–§15.3.12 of the spec's sheet but need a property, value, selector or layout mode this file does not list (or one
that parses but has no effect yet), so the UA sheet leaves them out or approximates them. Each is listed per rule, or
per category for the presentational hints, the quirks-mode rules and the prose-only rules. Like the other
simplifications, each is declared here rather than dropped silently.

| gap                                                                                                                                                                                                                                                                                                                                           | behaviour instead                                                                                                    | tracked for |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ----------- |
| `:link` / `:visited` (no link-state pseudo-class)                                                                                                                                                                                                                                                                                             | `a[href]` stands for `:link`; a visited link keeps `#0000ee` instead of `#551a8b`                                    | v0.7        |
| `display: table` and the `table-*` values (§15.3.8)                                                                                                                                                                                                                                                                                           | `table`, `caption`, `thead`, `tbody`, `tfoot`, `tr`, `td`, `th` stack as blocks; `col` / `colgroup` generate nothing | v0.7        |
| `vertical-align` (`sub`, `sup`; `middle` and `valign` on table parts, §15.3.8)                                                                                                                                                                                                                                                                | `<sub>` / `<sup>` sit on the baseline; table cells stack as blocks, so there is nothing to align                     | v0.7        |
| `font-size: smaller` / `larger` (`sub`, `sup`, `small`, `big`)                                                                                                                                                                                                                                                                                | the element keeps its parent's font size                                                                             | v0.7        |
| `text-decoration: dotted underline` (`abbr[title]`, `acronym[title]`)                                                                                                                                                                                                                                                                         | no decoration                                                                                                        | v0.7        |
| `::before` / `::after` with `content: open-quote` (`q`)                                                                                                                                                                                                                                                                                       | no quotation marks are generated                                                                                     | v0.7        |
| `:not()` (`[hidden]:not([hidden=until-found i]):not(embed)`)                                                                                                                                                                                                                                                                                  | `[hidden]` hides every element, including `hidden=until-found` and `<embed hidden>`                                  | v0.7        |
| `display: list-item` on `details > summary:first-of-type`                                                                                                                                                                                                                                                                                     | `<summary>` is a block with no disclosure marker; `<details>` shows its whole content                                | v0.7        |
| `display: ruby` / `ruby-text` (`ruby`, `rt`)                                                                                                                                                                                                                                                                                                  | no ruby layout: the `<rt>` annotation follows its base text in normal flow                                           | v0.7        |
| `cursor: pointer` and `:link:active { color: #ff0000 }`                                                                                                                                                                                                                                                                                       | the pointer never changes over a link; `:active` parses but never matches, so an active link keeps its colour        | v0.7        |
| `display: contents` (`slot`, §15.3.3)                                                                                                                                                                                                                                                                                                         | `<slot>` generates a box of its own                                                                                  | v0.7        |
| `:popover-open` and the popover box (`[popover]`, §15.3.3)                                                                                                                                                                                                                                                                                    | a closed popover is laid out like any element                                                                        | v0.7        |
| the open `<dialog>` box (`position: absolute`, `margin: auto`, `width: fit-content`, …), `dialog:modal` and `::backdrop` (§15.3.3)                                                                                                                                                                                                            | an open `<dialog>` is a plain block in normal flow, with no backdrop                                                 | v0.7        |
| `:focus-visible { outline: auto }` (§15.3.4)                                                                                                                                                                                                                                                                                                  | no focus ring is drawn                                                                                               | v0.7        |
| §15.3.5 bidirectional text (`:dir()`, `unicode-bidi` on `bdi`, `bdo`, `[dir]`, …)                                                                                                                                                                                                                                                             | no bidi isolation or override                                                                                        | v0.7        |
| `li { text-align: match-parent }` and `list-style-type` per nesting depth (§15.3.7)                                                                                                                                                                                                                                                           | `<li>` inherits `text-align` as is; markers are not generated (see `display`)                                        | v0.7        |
| presentational hints of §15.3.2–§15.3.12, attribute-driven: `body[bgcolor]` / `[text]` / `[link]` / margins, `font[color|face|size]`, `br[clear]`, `ol[type]` / `ol[start]` / `li[type|value]`, `td[nowrap]`, `table[border]`, `[align]` / `[valign]` on table parts, `legend[align]`, … (and the spec's ASCII-case-insensitive `i` matching) | the attributes are ignored; only `div[align]` is honoured, and attribute values match case-sensitively               | v0.7        |
| table parts under `[hidden]` keep their table display with `visibility: collapse` (§15.3.8)                                                                                                                                                                                                                                                   | a hidden `<tr>`, `<thead>`, … generates no box, like any `[hidden]` element                                          | v0.7        |
| `white-space: pre-wrap` (`pre[wrap]`, §15.3.3; `textarea`, §15.3.10)                                                                                                                                                                                                                                                                          | `<pre wrap>` keeps `pre`; a `<textarea>` keeps the `normal` it inherits                                              | v0.7        |
| `border-spacing`, `border-collapse` (`table`, §15.3.8)                                                                                                                                                                                                                                                                                        | no table layout, so no spacing between cells                                                                         | v0.7        |
| `text-indent`, `text-shadow`, `appearance` (`table`, §15.3.8; controls, §15.3.10)                                                                                                                                                                                                                                                             | not resetting them is invisible: none of the three is supported anywhere                                             | v0.7        |
| `input:not(…) { overflow: clip !important }` and the `align-content` centring of controls (§15.3.10)                                                                                                                                                                                                                                          | a control's content may overflow its box; a control's label sits at its top                                          | v0.7        |
| the `hr` rule (§15.3.11)                                                                                                                                                                                                                                                                                                                      | owned by #82; `hr` only gets its `display: block` here                                                               | #82         |
| quirks-mode rules (§15.3.3 `form`, §15.3.7 `li`, §15.3.8 `table` fonts, §15.3.9, §15.3.10 controls)                                                                                                                                                                                                                                           | there is no quirks mode: every document is styled in no-quirks mode                                                  | v0.7        |
| prose-only rules: `th` centring when its parent's `text-align` is `initial` (§15.3.8), ruby anonymous boxes (§15.3.4), `br` / `wbr` `display-outside` (§15.3.4, #82)                                                                                                                                                                          | a `<th>` starts at the left; no ruby boxes; `<br>` is #82's                                                          | v0.7        |
| `embed[hidden] { display: inline; width: 0; height: 0 }` and `@media (scripting) { noscript { display: none } }` (§15.3.1)                                                                                                                                                                                                                    | a hidden `<embed>` generates no box; scripting is off, so `<noscript>` renders, as the spec intends                  | v0.7        |
| `counter-reset: list-item` (`ol`, `ul`, `menu`, §15.3.7)                                                                                                                                                                                                                                                                                      | no list counters; markers are not generated (see `display`)                                                          | v0.7        |
| `:autofill { field-sizing: fixed !important }`, `input[type=image i] { overflow-clip-margin: 0 }` (§15.3.10)                                                                                                                                                                                                                                  | neither property exists here                                                                                         | v0.7        |
| `fieldset { min-inline-size: min-content }` (§15.3.12)                                                                                                                                                                                                                                                                                        | `min-width` refuses `min-content`; a fieldset can shrink below its content                                           | v0.7        |

## Selectors

Written in the `E` / `F` element notation of the CSS specifications: one row is one grammatical form, not one example.
`@media` is listed here because §2.8's table puts it in the same column — it gates a rule the same way a selector
chooses its subjects.

| token                | since | notes                                                                                                                                                                                                              |
| -------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `E`                  | B1    | type selector, ASCII-lowercased for HTML; specificity `(0,0,1)`                                                                                                                                                    |
| `*`                  | B1    | universal; specificity `(0,0,0)`                                                                                                                                                                                   |
| `.class`             | B1    | matches a whole name in the whitespace-separated `class` list; `(0,1,0)`                                                                                                                                           |
| `#id`                | B1    | matches the `id` attribute exactly; `(1,0,0)`                                                                                                                                                                      |
| `[attr]`             | B1    | presence, whatever the value; `(0,1,0)`                                                                                                                                                                            |
| `[attr=value]`       | B1    | exact value, quoted or bare; `^=` / `$=` / `*=` / `~=` are refused                                                                                                                                                 |
| `E, F`               | B1    | selector list; refused members are skipped with a note, the rule keeps the ones that parse (deviates from Selectors L4 §3.1 — see `parser/selectors.rs`); the rule is dropped whole only when **no** member parses |
| `E F`                | B1    | descendant combinator                                                                                                                                                                                              |
| `E > F`              | B1    | child combinator                                                                                                                                                                                                   |
| `E + F`              | B1    | next-sibling combinator; counts element siblings only                                                                                                                                                              |
| `E ~ F`              | B1    | subsequent-sibling combinator                                                                                                                                                                                      |
| `:hover`             | B1    | parses and weighs `(0,1,0)`; **never matches** — a `DomSnapshot` has no interaction state                                                                                                                          |
| `:active`            | B1    | parses and weighs; never matches                                                                                                                                                                                   |
| `:focus`             | B1    | parses and weighs; never matches                                                                                                                                                                                   |
| `:first-child`       | B1    | 1-based among **element** siblings                                                                                                                                                                                 |
| `:last-child`        | B1    | 1-based among element siblings                                                                                                                                                                                     |
| `:nth-child()`       | B1    | `an+b`, `odd`, `even`, `n`, `-n+3`, a bare integer                                                                                                                                                                 |
| `@media (min-width)` | B1    | evaluated by the producer via `StyleSheetSet::matching_viewport`, never by the resolver                                                                                                                            |
| `@media (max-width)` | B1    | same; a rule still carrying a condition is skipped by the cascade                                                                                                                                                  |

Declared **out** for v0.5, and refused with a note rather than ignored:

| form                                        | why                                                          |
| ------------------------------------------- | ------------------------------------------------------------ |
| `:has()`                                    | needs reverse matching; cost is out of proportion (§2.8:344) |
| `:not()`, `:nth-of-type()`, every other `:` | not in the §2.8 column; arrives behind `PseudoClass`         |
| `::before`, `::after`                       | generate a box with no node; v0.7 (§2.8:346)                 |
| `svg\|rect` and every namespace             | no foreign content until v1.0 (§2.8:345)                     |
| `[attr^=v]`, `[attr$=v]`, `[attr*=v]`       | substring matchers are not in the §2.8 column                |
| `@supports`, `@font-face`, `@import`        | §2.8:347; the block is skipped whole and noted               |
| `@keyframes`                                | §2.8:347; animation has no phase before v0.7                 |
