# ADR-0026: `core/ui` and `core/browser` Are Siblings Joined by an Effect Seam

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-10-23

---

## Context and Problem Statement

`alloy/src/application/event_loop/` was meant to be the one loop that owns the main thread (`ADR-0019`). It grew to
about twenty responsibilities, and `Session` alone holds sixteen fields: the document, its subresources, the omnibox,
focus, hit testing, link policy, navigation and its supersession, fetch threads, error-page generation and frame
composition. Two review rounds (#106, #107) each pulled one piece out; the epic #110 is the structural fix. This ADR
records the shape of that fix.

---

## Decision Drivers

- The part of `Session` that answers "what document is this window showing" must be testable with no thread, no sleep
  and no window.
- Browser chrome (omnibox, hit testing, keymap) and the browsing context change for different reasons and must not know
  each other.
- `core/*` crates stay free of threads and OS I/O (`ADR-0010`, `ADR-0019`).

---

## Considered Options

- **Option 1**: Two sibling crates, `core/ui` (chrome) and `core/browser` (browsing context), each a pure state machine
  returning an effect enum; `alloy`'s executor mediates between them.
- **Option 2**: One `core/shell` crate holding both — the chrome would depend on documents and the document on the
  omnibox again.
- **Option 3**: Keep everything in `alloy` and only split files — the review feedback asked for the structural fix.

---

## Decision Outcome

Chosen option: **Option 1**.

- `core/ui` (#109) owns chrome widgets: render tree, hit testing, theme, omnibox, focus, router, keymap and commands.
- `core/browser` (#111) owns the browsing context: the document, its base URL, stylesheets and images, link regions,
  navigation, fetch-result application, subresource discovery and the error page.
- **Neither depends on the other.** Each returns an effect enum (`UiEffect`, `BrowserEffect`). `alloy`'s executor reads
  one and drives the other (`UiEffect::Navigate` → `browser.navigate`; `BrowserEffect::DocumentLoaded(url)` → the
  omnibox shows `url`). `arch-lint.toml` forbids the edge in both directions.
- `core/browser` is **pure**: it never spawns a thread, owns a channel or calls `HttpTransport`. It emits
  `BrowserEffect::Fetch(request, NavigationId)`; `alloy` runs it on a `std::thread` and feeds the stamped `FetchResult`
  back. `arch-lint.toml` (`browser-pure`) enforces it.
- `core/ui` never sees an `href` (#110 decision 12): it hit-tests opaque `ContentRegion`s and hands back a `RegionId`;
  `core/browser` resolves it and owns the link policy.
- Every `NavigationId`, `RegionId`, `FetchResult` and `BrowserEffect` carries a `ContextId`, so a context tree (#90) is
  a later addition, not a signature change. Only the root context exists.
- Layout and painting stay in `alloy/src/application/{pipeline,paint}.rs`; after each layout `alloy` calls
  `BrowsingContext::layout_completed(links)`, and the browser bumps the `LayoutEpoch` and mints the `RegionId`s.
- Moves update every importer in the same PR; there are no transitional re-exports (#110 decision 20).

### Crate map

| Crate          | Owns                                                                                              | May depend on                                  |
| -------------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `core/browser` | `BrowsingContext`, `Document`, `ImageStore`, `SubresourceDiscoverer`, `NavigationId`, `RegionId`, error page | `dom`, `html`, `css`, `network` and `graphics` (domain types) |
| `core/ui`      | render tree, hit test, theme, omnibox, focus, router, keymap (#109, #113)                         | not `core/browser`                             |

### Consequences

- Good: the supersession rules are proved with plain function calls (`core/browser` unit tests).
- Good: `Session` shrinks to a coordinator; the document and the chrome can evolve independently.
- Bad: `NavigationId` stamping reintroduces a comparison that `e832b6a`'s channel-per-navigation had removed; the trade
  is recorded in the `ADR-0019` amendment.
- Bad: a shared id type is needed where the two crates meet (`RegionId` is minted by `core/browser` and carried through
  `core/ui`'s tree). Until #109 lands it is a `core/browser` type consumed by `alloy`'s render tree; how `core/ui` names
  it without importing `core/browser` is #109's to settle.

---

## Links

- Epic #110 (decisions 2, 3, 4, 12–14, 20, 22, 23), #111, #109; `ADR-0010`, `ADR-0019`
