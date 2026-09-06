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
| `display`                    | B1    | keywords `none` / `block` / `inline` / `flex`; `flex` parses in B1 and lays out in B4                                                                                                      |
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
| `border`                     | ddg   | shorthand narrowed to the border **width** (the only geometry); `<line-style>` and colour scanned past; `none` / `0` → no border, no width given → dropped (`medium` is not representable) |
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

Every property above also accepts the CSS-wide keywords `initial` and `inherit` (B2, CSS Cascade L4 §7.1): `initial`
resets to the value `ComputedStyle::initial()` gives it, `inherit` copies the parent's computed value even for a
property that does not normally inherit. `unset` and `revert` are not recognised.

Declared **out** for v0.5, and refused with a note: `float`, `clear`, `visibility`, and every other property.
(`background` and `border` are now **in**, narrowed to one component each — see the rows above and
`PORT_SCHEMA_VERSION = 5`; the `background`/`border` _longhands_ other than `-width` stay out.) `!important` is parsed,
preserved on the declaration, **and** wins the cascade as of B2 (`plano:435-443`): CSS Cascade L4 §4.2's
origin/importance ordering, `User` origin included by construction even though nothing sources it yet.

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

### Font simplifications (fonts increment)

`font-family` is a `Copy` field of `ComputedStyle` (`core/css/src/domain/computed/font.rs`), which is copied per node
during layout, so the list is fixed-capacity rather than a `Vec`. Two cuts, silent (the value is parsed only at cascade
time, which has no `ParseNote` channel) and declared here in the same spirit as the Flexbox cuts:

| gap                                      | behaviour instead                                                                                                                                                 | tracked for |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| the list has no length limit in CSS      | at most `FontFamilyList::CAPACITY` (3) families are kept; a real chain ends in a generic and the provider default is a generic, so the dropped tail is equivalent | v0.7        |
| a family name has no length limit in CSS | a name longer than `FamilyName::CAPACITY` (23) bytes is truncated at a UTF-8 boundary                                                                             | v0.7        |

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
