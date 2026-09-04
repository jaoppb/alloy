# `core/css` support manifest

The declared cut of `docs/reports/IMPLEMENTACAO-DETALHADA-V0-5.md` §2.8, property by property and selector by selector.
Everything **not** listed here is refused by the parser and recorded as a `ParseNote` — never accepted and silently
ignored, which is the shrinkage this file exists to prevent (§2.8:350-354).

`core/css/tests/manifest_runner.rs` checks three things and fails loudly on any of them:

1. this file and `css::SUPPORTED_PROPERTIES` / `css::SUPPORTED_SELECTORS` name the same sets, **in both directions**;
2. every listed token has a probe that the parser really accepts, and — for a property — that really changes the
   computed style the cascade produces;
3. a battery of forms declared **out** is really refused, each with a note.

There is **no bless path**: this file is hand-maintained, because the `notes` column carries reasons no generator can
invent. A new supported token means editing the registry in `core/css/src/lib.rs`, this table, and the probe table in
the runner — all three, or CI is red.

`since` is the phase of `~/.claude/plans/…-fancy-dijkstra.md` that landed the token.

## Properties

The CSS properties the parser accepts inside a declaration block and the cascade resolves to a computed value. A
declaration naming anything else is dropped on its own, with a note, leaving the rest of its rule intact.

| token              | since | notes                                                                                  |
| ------------------ | ----- | -------------------------------------------------------------------------------------- |
| `display`          | B1    | keywords `none` / `block` / `inline` / `flex`; `flex` parses in B1 and lays out in B4  |
| `color`            | B1    | inherited; `#rgb`, `#rrggbb` and the 17 basic colour names — `rgb()` / `rgba()` are B2 |
| `background-color` | B1    | not inherited; same value grammar as `color`                                           |
| `margin`           | B1    | the 1–4 component shorthand (CSS Box Model §8.3)                                       |
| `margin-top`       | B1    | longhand; overwrites only its own side                                                 |
| `margin-right`     | B1    | longhand                                                                               |
| `margin-bottom`    | B1    | longhand                                                                               |
| `margin-left`      | B1    | longhand                                                                               |
| `padding`          | B1    | the 1–4 component shorthand                                                            |
| `padding-top`      | B1    | longhand                                                                               |
| `padding-right`    | B1    | longhand                                                                               |
| `padding-bottom`   | B1    | longhand                                                                               |
| `padding-left`     | B1    | longhand                                                                               |
| `font-size`        | B1    | inherited; `px` / `em` / `rem` / `%` / `pt`, and the unitless `0`                      |

Declared **out** for v0.5 B1, and refused with a note: `float`, `position`, `width`, `height`, `border`, `z-index`,
`box-sizing`, `flex-direction` and every other property. `!important` is _parsed and preserved_ on the declaration but
does not yet win the cascade — that is B2 (`plano:435-443`).

## Selectors

Written in the `E` / `F` element notation of the CSS specifications: one row is one grammatical form, not one example.
`@media` is listed here because §2.8's table puts it in the same column — it gates a rule the same way a selector
chooses its subjects.

| token                | since | notes                                                                                     |
| -------------------- | ----- | ----------------------------------------------------------------------------------------- |
| `E`                  | B1    | type selector, ASCII-lowercased for HTML; specificity `(0,0,1)`                           |
| `*`                  | B1    | universal; specificity `(0,0,0)`                                                          |
| `.class`             | B1    | matches a whole name in the whitespace-separated `class` list; `(0,1,0)`                  |
| `#id`                | B1    | matches the `id` attribute exactly; `(1,0,0)`                                             |
| `[attr]`             | B1    | presence, whatever the value; `(0,1,0)`                                                   |
| `[attr=value]`       | B1    | exact value, quoted or bare; `^=` / `$=` / `*=` / `~=` are refused                        |
| `E, F`               | B1    | selector list; one invalid member invalidates the whole rule (Selectors L4 §3.1)          |
| `E F`                | B1    | descendant combinator                                                                     |
| `E > F`              | B1    | child combinator                                                                          |
| `E + F`              | B1    | next-sibling combinator; counts element siblings only                                     |
| `E ~ F`              | B1    | subsequent-sibling combinator                                                             |
| `:hover`             | B1    | parses and weighs `(0,1,0)`; **never matches** — a `DomSnapshot` has no interaction state |
| `:active`            | B1    | parses and weighs; never matches                                                          |
| `:focus`             | B1    | parses and weighs; never matches                                                          |
| `:first-child`       | B1    | 1-based among **element** siblings                                                        |
| `:last-child`        | B1    | 1-based among element siblings                                                            |
| `:nth-child()`       | B1    | `an+b`, `odd`, `even`, `n`, `-n+3`, a bare integer                                        |
| `@media (min-width)` | B1    | evaluated by the producer via `StyleSheetSet::matching_viewport`, never by the resolver   |
| `@media (max-width)` | B1    | same; a rule still carrying a condition is skipped by the cascade                         |

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
