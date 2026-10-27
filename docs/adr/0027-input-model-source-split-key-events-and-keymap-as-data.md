# ADR-0027: Input Model — Source-Split Port Events and Keymap-as-Data

- **Status**: Accepted
- **Deciders**: Architecture Team
- **Date**: 2026-10-23

---

## Context and Problem Statement

Until `window::PORT_SCHEMA_VERSION = 4` the `winit` adapter turned each key press into exactly **one** of `ControlKey`,
`TextInput` or `Key` (`event_map.rs`, issue #112). The adapter therefore decided, before any policy ran, whether a press
was a shortcut or typing — and it decided wrongly:

- Alt+D produces text `"d"`, so it was reported as `TextInput` with no key and no modifiers. No shortcut can be bound to
  it (#117).
- `winit`'s `KeyEvent::text` ignores Ctrl on Linux (xkb), so Ctrl+L carries `"l"` and fails the same way (#117).
- `ModifiersChanged` was dropped; `alloy` rebuilt modifiers from raw `Key` events, which is wrong whenever a modifier
  was pressed while the window was unfocused.
- The shortcut table (Ctrl+L, Alt+D) was hardcoded in `alloy`, against `ADR-0003` (policy belongs to the Muscle).

Epic #110 decomposes the event loop and needs one decision on how input crosses the `window` port and where the
chord-versus-text decision lives.

---

## Decision Drivers

- The adapter is mechanism (`ADR-0003`): it reports what the platform said and never classifies intent.
- A key press has one cause, so it should be one event carrying everything about it.
- Shortcuts are user policy and must be overridable by a script (#50) without a script call per key press: the hot path
  has a 10 µs budget (`PRD-001`), and a failing script must not leave the browser without shortcuts (`ADR-0004`).
- The behaviour must be testable without a display server, and identically on Linux, Windows and macOS.

---

## Considered Options

- **Option 1**: Keep classification in the adapter and patch it (emit `Key` _and_ `TextInput` for a text-bearing press).
  Two events per press, ordering hazards, and the consumer still cannot tell which copy to act on.
- **Option 2**: One `KeyEvent` per physical press carrying logical key, physical key, text, modifiers and
  `Pressed | Repeated | Released`; `TextCommitted` only for input-method commits. A keymap — data, resolved in Rust —
  decides.
- **Option 3**: Option 2 with the keymap as a script callback invoked for every key press. Rejected: puts a script call
  on the hot path and removes the Rust default when the script fails.

---

## Decision Outcome

Chosen option: **Option 2**.

### Port events

- `WindowEvent::Key(KeyEvent)` is one physical press, auto-repeat or release. `KeyEvent` carries `logical`
  (`LogicalKey`, including `Character(InputText)`), `physical` (`KeyCode`, the layout-independent fallback for chords),
  `text: Option<InputText>`, `modifiers: Modifiers` and `state: KeyState`.
- `WindowEvent::TextCommitted(InputText)` is for input-method commits only — text with no single key behind it.
- `ControlKey` and key-produced `TextInput` are removed. There is no alias (epic decision 19).
- One `InputText` newtype (non-empty, no control characters) serves both payloads, so text is validated once at the port
  edge.
- A held key yields `Pressed`, then `Repeated`…, then `Released` — never repeated `Pressed`.

### `text` is `winit`'s `KeyEvent::text`

`text` is what the press would insert with the layout and Shift applied, and **ignoring Ctrl** on Linux. The adapter
does not use `text_with_all_modifiers`, and it does not strip text when Ctrl is held: Ctrl+L carries `"l"`. That is
intended — the consumer reads `modifiers` and drops text under a command modifier. An adapter that guessed would
reintroduce #117 on the next platform quirk.

### Modifiers and AltGr

The adapter stamps `Modifiers` on every `KeyEvent` from `ModifiersChanged`. `winit`'s `ModifiersState` has no AltGr bit,
so the adapter also tracks `NamedKey::AltGraph` press and release itself and sets `Modifiers::alt_graph()`. On Linux
this is the only signal. From the `winit` 0.30.13 source (not run on Windows hardware): on a Windows layout that has an
AltGr key, `winit` names the right Alt key `AltGraph` and clears Ctrl and Alt from `ModifiersState` while it is held, so
AltGr+Q arrives as `alt_graph()` with neither `control()` nor `alt()` — the same shape as Linux. What still arrives as
plain Ctrl+Alt on Windows is the _left_ Ctrl+Alt, which Windows also accepts as AltGr; router step 3 covers it. macOS
reports Option as Alt (no AltGraph key), see below.

Which physical modifier keys are down is recorded from the key events themselves, synthetic ones included, and cleared
when the window loses focus. A modifier key's own events follow the DOM convention for **every** modifier: its press
reports it held, its release reports it released unless the other side of the pair (left/right Shift, Ctrl, Alt, Super)
is still down. This does not depend on whether `winit` delivers `ModifiersChanged` before or after the key event, which
differs by platform (X11 stamps the state from just before the event).

### Synthetic key events

On X11 and Windows, `winit` synthesises a press for every key already held when the window gains focus and a release for
every key still down when it loses focus (`KeyboardInput::is_synthetic`). They are platform bookkeeping, not user
intent: a held letter would otherwise become an `Insert` into a focused omnibox, and a held Ctrl+L would fire `Focus`.
The adapter never reports them as `WindowEvent`s, but it still records them, because they say which keys are really
down.

### Keymap is data

A script runs once (on init and reload) and returns bindings; Rust stores a `Keymap` and resolves every key press
itself. There are zero script calls on the hot path, and a script failure keeps the Rust default keymap (`ADR-0004`).
The `Keymap`, the input router and `UiInput::{Command, Edit, Insert}` live in `core/ui` (epic #110, issue #113).

- **Chords.** A `KeyChord` is a modifier set plus a key, matched with the modifiers **exactly** (Ctrl+Shift+L is not
  Ctrl+L, and a chord that includes AltGr is a different chord). The key is matched on the logical key first (single
  characters case-folded) and on the physical key second, so a non-Latin layout still reaches Ctrl+L.
- **Commands.** `BrowserCommand` is a closed enum parsed from action names (`focus_omnibox`); the default keymap binds
  Ctrl+L, Super+L (Cmd+L on macOS, on every platform) and Alt+D to it, by logical and by physical key, and no Ctrl+Alt
  chord. The default is written in the same chord text a script supplies and parsed by the same code (`ctrl+l`,
  `ctrl+code:l`: `code:` names a Latin letter or digit by position).
- **The seam.** `Keymap::from_bindings` takes unvalidated `(chord text, action name)` pairs and returns the keymap with
  the `KeymapError`s of the rejected bindings. A malformed chord or an unknown action rejects that binding alone, a
  later binding of the same chord replaces an earlier one, and a set with no valid binding yields `Keymap::default()`.
  The composition root resolves the source once before the loop and on reload (hook: #115; script: #50).
- **Platform facts.** The router takes a `platform::HostOs` (`ADR-0022`) at construction; it is the only source of the
  two platform rules below, so every platform's behaviour is unit-tested on any host. The router decides what an event
  means; whether the omnibox consumes typing is the controller's call, and one click can return several `UiEffects`
  (ending address editing damages the chrome _and_ follows the link).

### Router order for a `Pressed` or `Repeated` key event

1. A keymap binding matches → the bound `Command`. Chords match the logical key first and the physical key as the
   fallback. This comes first so a chord that includes AltGr (Ctrl+AltGr+L) stays bindable.
2. `alt_graph` and `text` → insert the text.
3. Windows only, exactly Ctrl+Alt and `text` → insert the text (the fallback for the left Ctrl+Alt that Windows accepts
   as AltGr).
4. Ctrl, Super or Alt held without AltGr → dropped; nothing is inserted (an unbound Ctrl+K types nothing).
5. Otherwise `text` → insert the text.

Shift never blocks insertion. `Released` events never produce input, and a held (`Repeated`) binding fires once, on the
press; a held editing key or letter repeats. Named editing keys under Ctrl, Super or Alt are dropped unless the keymap
binds them. Typing and editing reach only a focused omnibox; a command reaches the browser whatever has focus.

On macOS, Option+letter must insert its composed text and the binding for "focus the address bar" is Super+L. `winit`
reports Option as Alt there (no AltGraph key), so the adapter sets `alt()`, not `alt_graph()`. The router therefore does
not count Alt as a command modifier when `HostOs` is `MacOs`: Option+letter inserts (step 5) while Cmd and Ctrl still
drop. The Windows rule of step 3 is likewise gated on `HostOs::Windows`. This port cannot branch on `target_os`.

### Window mutation commands

The same version bump adds `WindowSystem::set_title(WindowTitle)` and `WindowSystem::set_cursor(CursorIcon)`: commands
(no return value, silent no-op before a window exists), recorded by `HeadlessWindowSystem`.

### Consequences

- Good: Alt+D and Ctrl+L are bindable; the adapter no longer owns any shortcut or typing policy.
- Good: modifiers come from the platform, so a modifier held across a focus change is correct.
- Good: the whole mapping is unit-testable through `KeyInput`, because `winit::event::KeyEvent` cannot be constructed
  outside `winit`.
- Bad: every consumer must read `modifiers` before treating `text` as typing. `core/ui`'s `InputRouter` is the one place
  that does; `alloy`'s event loop forwards `KeyEvent`s whole (`arch-lint` rule `event-loop-no-key-policy`).
- Bad: exact-modifier matching means the default Ctrl+L does not fire while AltGr is also held; that chord types its
  text unless a keymap binds it.
- Bad: `PORT_SCHEMA_VERSION` 3 → 4 breaks every `WindowSystem` implementer and every `match` on `WindowEvent`.
- Bad: Alt and AltGr text payloads, and the `winit` behaviours above, are read from the 0.30.13 source and unverified on
  Windows and macOS (no hardware in CI); the `event_map` tests encode the expected mapping and a manual check is tracked
  on #112.

---

### Links

- `docs/architecture/window-system-port-contract.md` §4, `docs/requirements/PRD-010-window-system-and-presenter-port.md`
  §6
- `ADR-0003`, `ADR-0004`, `ADR-0011`, `ADR-0019`, epic #110, issues #112, #113, #117
