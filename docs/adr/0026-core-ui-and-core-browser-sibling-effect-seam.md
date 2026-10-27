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

- `core/ui` (#109) owns chrome widgets: render tree, hit testing, theme, omnibox (state, controller, painter and the
  `OmniboxClassifier` that left `network`), focus, router, keymap and commands.
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
- **`core/region` is the one thing the two siblings share** (#109; see
  [the amendment](#amendment-109-where-regionid-lives)). `ContextId`, `LayoutEpoch`, `RegionId` and `ContentRegion` live
  in this leaf crate; `core/browser` mints and resolves them, `core/ui` hit-tests them, `alloy` passes them through
  untouched.
- Layout and painting stay in `alloy/src/application/{pipeline,paint}.rs`; after each layout `alloy` calls
  `BrowsingContext::layout_completed(links)`, and the browser bumps the `LayoutEpoch` and mints the `RegionId`s.
- Moves update every importer in the same PR; there are no transitional re-exports (#110 decision 20).

### Crate map

| Crate          | Owns                                                                                              | May depend on                                  |
| -------------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `core/browser` | `BrowsingContext`, `Document`, `ImageStore`, `SubresourceDiscoverer`, `NavigationId`, link hrefs, error page | `dom`, `html`, `css`, `network` and `graphics` (domain types), `region` |
| `core/ui`      | render tree, hit test, theme, omnibox + classifier, focus, router, keymap (#109, #113)            | `graphics`, `window` and `network` (domain types), `region` — not `core/browser` |
| `core/region`  | `ContextId`, `LayoutEpoch`, `RegionId`, `ContentRegion` — the seam vocabulary (#109)              | `graphics` (`Rect`) only                       |

### Consequences

- Good: the supersession rules are proved with plain function calls (`core/browser` unit tests).
- Good: `Session` shrinks to a coordinator; the document and the chrome can evolve independently.
- Bad: `NavigationId` stamping reintroduces a comparison that `e832b6a`'s channel-per-navigation had removed; the trade
  is recorded in the `ADR-0019` amendment.
- Bad: a shared id type is needed where the two crates meet (`RegionId` is minted by `core/browser` and carried through
  `core/ui`'s tree). Settled by #109 with a third, leaf crate — see the amendment.
- Bad: `core/region` is a crate for four small value types; the price of keeping the siblings independent.

---

## Amendment (#109): where `RegionId` lives

Epic #110 decision 23 had #109 mint `RegionId` with a placeholder `ContextId` defined in `core/ui`, to be moved when
issue #111 landed. It landed first and put `ContextId` and `RegionId` in `core/browser`, which `core/ui` must not depend
on (decision 3), so the placeholder was no longer the question. Options:

- **A shared leaf crate (chosen).** `core/region` holds `ContextId`, `LayoutEpoch`, `RegionId` and `ContentRegion`. Both
  siblings depend on it; `alloy` hands the browser's regions to the render tree and the winning `RegionId` back to the
  browser with no conversion and no lookup table. The constructors are public — `core/ui`'s tests and `core/browser`
  both build values — so "only `core/browser` mints" is a convention the compiler no longer checks, and `arch-lint`
  cannot check it either (`restrict-use` matches imports, not method calls). `ContentRegions::href` still refuses an id
  from another context or an earlier epoch (`RegionError`), which is what decision 13 relies on. An id forged for the
  current layout would resolve to a real link of that layout and no other: no new capability, but not impossible. If
  that is ever too weak, the next step is a `RegionMint` owned by `core/browser`.
- **`core/ui` owns its own opaque `RegionId`.** `alloy` would convert every `browser::ContentRegion` on each layout and
  keep a parallel `Vec<browser::RegionId>` to map a hit back; that table can drift from the tree and duplicates the id
  concept. Rejected.
- **Put the ids in `graphics`.** No new crate, but browsing-context identity has nothing to do with a display list.
  Rejected.

`ContentRegion` moved too, so there is one type for "where a link is and its id" instead of two with the same shape.
`LinkTarget` and `LinkTargets` — which carry the `href` — stay in `core/browser`, so `core/ui` still never sees one.
`arch-lint.toml` holds the edges: `ui-isolated` and `region-isolated` (and `browser-isolated`, which already denied
`ui`). `UiRenderTree` keeps `UiHitTarget::PageLink(RegionId)` / `PageContent` and `UiNodeKind::Link(RegionId)` — the
names #109 moved, unchanged — so the move is behavior-preserving.

**What the later issues must know.**

- #113 (router): the hit target is `UiHitTarget::PageLink(RegionId)`; the executor calls `browser.follow_link(id)`.
  Import `RegionId` from `region`, not `browser`. The old router order of decision 18 is superseded by `ADR-0027` and is
  not encoded in `core/ui`.
- #115 (executor): `alloy` depends on `ui`, `browser` and `region`; `UiError` converts into
  `AlloyError::InvalidDimensions`; the chrome font id is `ui::CHROME_FONT`, pinned equal to `alloy`'s `DEFAULT_FONT` by
  `alloy/tests/omnibox_golden.rs`.
- `UiRenderTree::fallback` still reports `content_size() == Err`; fixing that is #108/#118, not this move.

---

## Links

- Epic #110 (decisions 2, 3, 4, 11–14, 20, 22, 23), #111, #109; `ADR-0010`, `ADR-0019`, `ADR-0027`
