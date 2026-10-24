# SPDD Analysis: `core/browser` - the pure browsing context (issue #111)

## Original Business Requirement

[jaoppb/alloy#111](https://github.com/jaoppb/alloy/issues/111), sub-issue of the event-loop epic #110: everything
`Session` holds about _the document_ becomes a pure state machine in a new `core/browser` crate that never spawns a
thread and returns `BrowserEffect`s for `alloy` to execute. Acceptance: ADR-0026 + CLAUDE.md crate map; ADR-0019
amendment for `NavigationId`; an `arch-lint.toml` rule keeping threads, `mpsc` and `HttpTransport` out; a pure,
sleepless test that a late `complete` is dropped; `ContextId` on every id/effect/result; a stale-`LayoutEpoch`
`follow_link` test; the error page renders `<script>&` as text; entities under ~100 lines; no behaviour change
(`alloy/tests/golden/*`, `navigation_integration.rs`).

## Current State (as found, base `1b52465`)

- `Session` (16 fields) owned `dom_tree`, `base_url`, `extra_sheets`, `images`, `links`, `dirty`, and an
  `inbox`/`outbox` pair **replaced on every navigation** so a worker of the page the user left could not report back.
- Fetch-result application (`drain_messages`, `apply`, `load_document`, `absorb_stylesheet`, `spawn_subresources`) and
  link policy lived in `session/{mod,messages}.rs`; the error page was a `format!` with `&` and `<` escaped.
- `SubresourceDiscoverer` and `ImageStore` lived in `alloy/src/application`.
- Hit testing returned the raw `href`, resolved against `base_url` on click - also after a document swap and before the
  next relayout, so a click could follow the previous page's link against the new page's base.

## Decisions

1. **Crate shape** - `core/browser` with `domain/` (ids, `Document`, `ContentRegions`, requests, effects, stats, link
   policy), `application/` (`BrowsingContext`, `SubresourceDiscoverer`, `error_page`) and `infrastructure/`
   (`MarkupDiscoverer`). Depends on `dom`, `html`, `css`, `network`, `graphics` (domain types only).
2. **Supersession by `NavigationId`** (#110 decision 4) - one id per navigation, stamped on its fetches; `complete`
   drops a non-latest id. Recorded as an ADR-0019 amendment. A _genesis_ id (sequence 0, never issued) keeps `latest()`
   total.
3. **`FetchResult` carries its `NavigationId`** (and so its `ContextId`), so `complete(result)` takes one argument
   rather than `complete(NavigationId, FetchResult)`: the stamp cannot disagree with the payload.
4. **Effects** - `NavigationStarted`, `Fetch`, `DocumentLoaded`, `ContentDamaged`, each naming its context.
   `Damage(Content)` is `ContentDamaged` until #108 defines `Damage`; `TitleChanged` is left to #84.
5. **Regions** - `RegionId { context, epoch, index }`; `ContentRegions` bumps the epoch on `replace` and `invalidate`.
   `layout_completed(links) -> LayoutEpoch`, `content_regions()`, `follow_link(RegionId)`.
6. **Error page** - embedded `error.html`; the message is a DOM text node appended to `#error-message`.
7. **`alloy` side** - `worker.rs` becomes the executor of `FetchRequest`; `Session` keeps one channel, executes effects,
   and builds `LoopStats` from `BrowsingContext::stats()`. The render tree and hit target carry `RegionId`
   (`UiHitTarget::PageLink(RegionId)`), so `href`s no longer reach the chrome.
8. **`BrowserServices` shares its discoverer** as `Arc<D>` so the context and the services hold the same handle.

## Open points handed on

- `resize` and `cursor_for(RegionId)` from the issue's command list are not built: the viewport is chrome geometry the
  compositor owns (#108/#115) and the cursor needs window port v4 (#68). Nothing would read them today.
- `core/ui` (#109) must name `RegionId` without depending on `core/browser` (decision 3 of #110 makes them siblings).
