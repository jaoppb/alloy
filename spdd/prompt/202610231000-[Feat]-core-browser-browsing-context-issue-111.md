# `core/browser`: the pure browsing context with `NavigationId` supersession (issue #111)

## Requirements

1. `core/browser` contains no `std::thread`, `std::sync::mpsc` or `HttpTransport` use — enforced by `arch-lint.toml`.
2. A `complete` for a superseded navigation is dropped and logged — pure unit test, no sleeps.
3. `ContextId` is carried by `NavigationId`, `RegionId`, `FetchResult` and every `BrowserEffect`.
4. A `follow_link` with a stale `LayoutEpoch` is ignored — pure unit test.
5. The error page renders `<script>&` as text; no markup is injected.
6. No behaviour change: `alloy/tests/golden/*` and `navigation_integration.rs` unchanged, `just gate` passes.

## Entities

- `ContextId`, `NavigationId`, `Navigations`, `LayoutEpoch`, `RegionId`, `RegionError`, `LinkTarget`, `ContentRegion`,
  `ContentRegions`, `Document`, `StylesheetRejected`, `LinkDestination`, `ContextStats` — `core/browser/src/domain`.
- `FetchRequest`, `FetchPayload`, `FetchResult`, `FetchFailure`, `LoadedDocument`, `BrowserEffect` — the effect seam.
- `StylesheetRequest`, `ImageRequest`, `SubresourceRequest`, `Subresources`, `ImageStore` — moved from `alloy`.
- `BrowsingContext<D>`, `SubresourceDiscoverer`, `document_base_url`, `error_page` — `application/`.
- `MarkupDiscoverer` — `infrastructure/`.

## Approach

- Commands in, effects out: `navigate`, `follow_link`, `complete`, `layout_completed`; no thread, channel or transport.
- Results are stamped with the `NavigationId` of the navigation that started them; only the latest id is applied.
- Link regions are addressed by `RegionId { context, epoch, index }`; the epoch bumps on every layout and document swap.
- `alloy` keeps a thin executor (`worker.rs`, `Session::execute_browser_effects`) over today's thread-per-fetch workers.

## Operations

1. Create the crate; move subresource discovery and `ImageStore`; add the `arch-lint.toml` scopes and rules.
2. Add ids, `Navigations`, `ContentRegions`, `Document`, fetch/effect vocabulary and link policy, each with tests.
3. Add `error_page` and `BrowsingContext` (`completion.rs` for `complete`), test-first, with supersession and
   stale-epoch tests ported from `event_loop/tests.rs`.
4. Rewire `alloy`: `worker.rs` executes `FetchRequest`; `Session` holds a `BrowsingContext`; the render tree and hit
   target carry `RegionId`; delete `session/messages.rs`; update every importer (no re-exports).
5. ADR-0026, the ADR-0019 amendment, the ADR index row, CLAUDE.md crate map and Current State.

## Norms

- Object Calisthenics: no `else`, first-class collections, one indentation level, no naked primitives, entities < ~100
  lines. `thiserror` for errors, `tracing` for logging, `forbid(unsafe_code)`.
- `just gate` passes; strict clippy; `arch-lint` clean.

## Safeguards

- `core/browser` never names `window`, `engine`, `core/ui` or a script runtime (`browser-isolated`).
- A stale `RegionId` or `NavigationId` is logged and ignored, never resolved against the new document.
