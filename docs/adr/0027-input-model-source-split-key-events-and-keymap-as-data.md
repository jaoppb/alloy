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
so the adapter also tracks `NamedKey::AltGraph` press and release itself and sets `Modifiers::alt_graph()` (Linux only:
macOS reports Option as Alt, see the router rules below). The tracked state is cleared when the window loses focus.
Windows cannot tell AltGr from Ctrl+Alt; there `alt_graph()` is never set and the fallback below applies.

### Keymap is data

A script runs once (on init and reload) and returns bindings; Rust stores a `Keymap` and resolves every key press
itself. There are zero script calls on the hot path, and a script failure keeps the Rust default keymap (`ADR-0004`).
The `Keymap`, the input router and `UiInput::{Command, Edit, Insert}` live in `core/ui` (epic #110, issue #113).

### Router order for a `Pressed` or `Repeated` key event

1. `alt_graph` and `text` → insert the text.
2. Windows only, exactly Ctrl+Alt and `text` → insert the text (the AltGr fallback).
3. A keymap binding matches → the bound `Command`. Chords match the logical key first and the physical key as the
   fallback.
4. Ctrl, Super or Alt held without AltGr → dropped; nothing is inserted (an unbound Ctrl+K types nothing).
5. Otherwise `text` → insert the text.

Shift never blocks insertion. `Released` events never produce input. Named editing keys under Ctrl, Super or Alt are
dropped unless the keymap binds them (the interim `alloy` mapping already applies this to both typing and editing). On
macOS, Option+letter must insert its composed text and the binding for "focus the address bar" is Super+L. `winit`
reports Option as Alt there (no AltGraph key), so the adapter sets `alt()`, not `alt_graph()`: the router needs a
platform fact from `core/platform` (`ADR-0022`) to apply that rule, which is left to the router (issue 113). This port
cannot branch on `target_os`.

### Window mutation commands

The same version bump adds `WindowSystem::set_title(WindowTitle)` and `WindowSystem::set_cursor(CursorIcon)`: commands
(no return value, silent no-op before a window exists), recorded by `HeadlessWindowSystem`.

### Consequences

- Good: Alt+D and Ctrl+L are bindable; the adapter no longer owns any shortcut or typing policy.
- Good: modifiers come from the platform, so a modifier held across a focus change is correct.
- Good: the whole mapping is unit-testable through `KeyInput`, because `winit::event::KeyEvent` cannot be constructed
  outside `winit`.
- Bad: every consumer must read `modifiers` before treating `text` as typing. Until the router lands (#113), `alloy`
  carries an interim `KeyEvent` → `OmniboxInput` mapping that applies steps 3–5 of the rule.
- Bad: `PORT_SCHEMA_VERSION` 3 → 4 breaks every `WindowSystem` implementer and every `match` on `WindowEvent`.
- Bad: Alt and AltGr text payloads are unverified on Windows and macOS (no hardware in CI); the `event_map` tests encode
  the expected mapping and a manual check is tracked on #112.

---

### Links

- `docs/architecture/window-system-port-contract.md` §4, `docs/requirements/PRD-010-window-system-and-presenter-port.md`
  §6
- `ADR-0003`, `ADR-0004`, `ADR-0011`, `ADR-0019`, epic #110, issues #112, #113, #117
