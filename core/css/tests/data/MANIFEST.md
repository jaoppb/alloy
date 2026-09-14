# `core/css` support manifest

The CSS properties `core/css` resolves to a computed value, and the selector forms it matches against a `DomSnapshot`.
`core/css/tests/manifest_runner.rs` asserts this file and the crate's `SUPPORTED_PROPERTIES` / `SUPPORTED_SELECTORS`
registries agree **in both directions**: the build fails if the code supports something this file omits, or this file
lists something the code does not support. Regenerate with `UPDATE_MANIFEST=1 cargo test -p css --test manifest_runner`
(then run `pnpm format:md`).

## Properties

- `display`
- `color`
- `background-color`
- `margin`
- `padding`
- `font-size`

## Selectors

None yet — the selector engine arrives in B1.
