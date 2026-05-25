# Vim Binding for the Lisette Playground

**Date:** 2026-05-25
**Status:** Design approved, awaiting implementation plan
**Branch:** `vim-binding`

## Goal

Add modal editing (Vim) to the Monaco-based Lisette playground so the editor is usable with vim keys, while preserving the existing one-click toolbar UX for users who do not opt in.

## Decisions

1. **Vim only.** Use the `monaco-vim` library. Helix bindings are out of scope (no Monaco adapter exists; would require implementing a selection-first modal layer from scratch).
2. **Toggle, default OFF, persisted.** A toolbar button toggles vim mode; the state is stored in `localStorage` and restored on next visit.
3. **Status line under the editor.** A dedicated `<div>` directly below `editor-container` hosts the vim mode label (`-- NORMAL --`, etc.) and the ex/search input — the conventional vim layout.
4. **Four ex commands mapped to playground actions:** `:w` / `:fmt` → Format, `:run` / `:r` → Run, `:vim` → turn vim mode off.
5. **`Ctrl+Alt+V` toggles vim mode** in addition to the toolbar button.
6. **Toggle button hidden on mobile** (`max-width: 640px`). Mobile users continue with the existing touch-friendly toolbar; vim mode is a desktop-keyboard feature.

## Architecture

### Module layout

```
playground/
├── package.json             ← add dependency: monaco-vim (not currently installed)
├── index.html               ← add toggle button + vim-statusbar div
├── src/
│   ├── main.ts              ← toggle owner, persistence, action wiring
│   ├── style.css            ← styles for toggle button, statusbar, mobile media query
│   └── editor/
│       ├── index.ts         ← UNCHANGED (no vim awareness)
│       └── vim.ts           ← NEW: monaco-vim wrapper, ex commands
```

`vim.ts` is the only file that imports `monaco-vim`. `main.ts` treats vim as a layer that gets attached/detached on top of a fully-configured editor, never inside `setupEditors()`. This keeps `editor/index.ts` free of toggle state and action callbacks (which would otherwise leak across the boundary).

### Dependency flow

```
main.ts
  ├── owns: toggle button state, localStorage key, Ctrl+Alt+V shortcut
  ├── provides actions object → vim.ts (format, run, disable)
  └── calls: setupVim(editor, statusBarEl, actions) / dispose()

vim.ts
  ├── owns: monaco-vim instance lifecycle, ex command registration
  └── knows nothing about: DOM buttons, localStorage, status indicator text
```

The action callbacks (`format`, `run`, `disable`) are the same closures already bound in `main.ts` for the toolbar buttons — no duplication.

## DOM & UI changes

### `index.html`

Add toggle button in `toolbar-right`, **before** the status indicator:

```html
<div class="toolbar-right">
  <button id="btn-vim-toggle" class="btn btn-ghost" title="Toggle Vim mode (Ctrl+Alt+V)" aria-pressed="false">
    <span class="vim-toggle-label">Vim</span>
  </button>
  <span id="status-indicator" class="status-idle">Ready</span>
</div>
```

Add status bar inside `editor-pane`, after `editor-container`:

```html
<div id="editor-pane">
  <div id="editor-container"></div>
  <div id="vim-statusbar" hidden></div>
</div>
```

### `style.css`

- `#editor-pane` — change to `display: flex; flex-direction: column`. `#editor-container` gets `flex: 1`. `#vim-statusbar` takes natural height when visible.
- `#btn-vim-toggle` — matches existing `.btn-ghost` look (toolbar already has a "ghost" style; reuse it). When `aria-pressed="true"`, apply a single accent color (no full button restyle) so the active state reads at a glance.
- `#vim-statusbar` — `font-family: monospace`, `font-size: 12px`, `padding: 4px 8px`, background/border matching the editor theme. Flex row layout (monaco-vim inserts its own children).
- `@media (max-width: 640px)` — `#btn-vim-toggle { display: none }` and `#vim-statusbar { display: none !important }` (override the `hidden` toggle while on mobile).

## State management

### Persistence

```ts
const VIM_KEY = "lisette-playground:vim-mode";
// values: "on" | "off" | null (null → treat as "off")
```

Namespaced prefix `lisette-playground:` leaves room for future settings (e.g., theme override) without colliding.

### Initial load (`main.ts`)

1. `setupEditors()` returns `mainEditor`.
2. Read `localStorage[VIM_KEY]`; coerce to boolean `enabled`.
3. Sync toggle button: `aria-pressed = enabled`, optionally label state.
4. If `enabled`: call `setupVim(mainEditor, statusBarEl, actions)`, store returned handle. Show statusbar (`hidden = false`).
5. Wire toggle button, `Ctrl+Alt+V`, and `:vim` ex command all to the same `toggleVim()` function.

### Toggle behavior

`toggleVim(next: boolean)`:
- `next === true` and currently off → create instance, unhide statusbar, write `"on"`, focus editor.
- `next === false` and currently on → call instance `dispose()`, hide statusbar, clear stored handle, write `"off"`, focus editor.
- `localStorage` write wrapped in try/catch so Safari private mode etc. degrade to in-memory only (no thrown errors).

### Why re-create on toggle

`monaco-vim` exposes `dispose()` which removes its key handlers and DOM children cleanly. There is no "pause" mode. Re-creating the instance on toggle is the simplest model with no leak risk.

## Ex commands

Registered inside `setupVim()` via the `CodeMirror.Vim.defineEx(name, shortName, handler)` API (exposed by monaco-vim as `VimMode.Vim`):

| Ex            | Action  | Notes                                                    |
| ------------- | ------- | -------------------------------------------------------- |
| `:write` / `:w`   | Format  | Closest playground analogue to vim's save                |
| `:fmt`        | Format  | Explicit alias for users uncomfortable repurposing `:w`  |
| `:run` / `:r` | Run     | Intentionally overrides vim's default `:r filename` (file system n/a) |
| `:vim`        | Disable vim mode | Escape hatch when the toggle button is offscreen |

When `:w` runs, the existing `setStatus("ok", "Formatted (:w)")` call updates the toolbar status indicator. This reuses the existing pattern where status text persists until the next action overwrites it — no separate timer or fade. Same pattern for `:run` (`setStatus` is already called inside the existing run flow).

The `dispose()` returned by `setupVim()` does **not** unregister ex commands — the `Vim` singleton is module-scoped in monaco-vim, so commands persist across re-instantiation. Calling `defineEx` again on re-toggle simply overwrites with the same handlers, which is harmless.

## Keyboard shortcut conflict review

| Shortcut       | Existing                        | Vim normal-mode meaning | Resolution                                      |
| -------------- | ------------------------------- | ----------------------- | ----------------------------------------------- |
| `Ctrl+Enter`   | Run (document-level listener)   | (none)                  | No conflict. Event bubbles to document past vim handler. |
| `Alt+Shift+F`  | Format (Monaco command)         | (none)                  | No conflict.                                    |
| `Ctrl+F`       | Monaco find widget              | Page down               | Vim wins in normal mode (expected). Users press `/` for search. Insert mode preserves find. |
| `Ctrl+R`       | Browser reload                  | Redo                    | Browser wins (expected and safe).               |
| `Ctrl+B/D/U`   | (none in app)                   | Vim paging              | No conflict.                                    |
| `Ctrl+Alt+V`   | (new)                           | (none)                  | No conflict.                                    |

The existing `document.addEventListener("keydown", ...)` for `Ctrl+Enter` sits at document level (bubble phase), so vim's editor-level handler sees the event first but does not consume `Ctrl+Enter` — it bubbles up and runs the existing `run()` callback. **No change to that listener is needed.**

## Mobile behavior

- Toggle button hidden via CSS at ≤ 640px.
- Status bar also force-hidden at ≤ 640px via media query (overriding any `hidden=false` set by toggle state).
- Vim instance lifecycle is **not** tied to viewport — if a user toggled on at desktop width and then resized below 640px, the instance keeps running but the UI surface disappears. Acceptable trade-off; resizing back restores the UI.

## Verification (manual)

The playground has no automated test suite. Verify with `npm run dev` against this checklist:

**Basic:**
- [ ] First load: vim OFF, button unpressed, statusbar hidden, editing works normally.
- [ ] Toggle ON: statusbar appears showing `-- NORMAL --`; `h/j/k/l`, `i`, `Esc` behave as vim.
- [ ] Toggle OFF: statusbar hidden, editing reverts to plain Monaco.
- [ ] Reload preserves last toggle state.
- [ ] `Ctrl+Alt+V` toggles identically to button.

**Ex commands:**
- [ ] `:w` and `:fmt` → format; status shows "Formatted (:w)" or similar.
- [ ] `:run` / `:r` → run.
- [ ] `:vim` → turn vim off.

**Regression:**
- [ ] With vim OFF, every existing feature works identically (Run/Format/Check/Share/tabs/drawer/resizer/diagnostics jump).
- [ ] With vim ON in insert mode, `Ctrl+Enter` still runs.
- [ ] With vim ON in normal mode, `Alt+Shift+F` still formats.
- [ ] Toggling while WASM is loading does not error.

**Visual/accessibility:**
- [ ] Dark↔light system theme switch repaints statusbar to match.
- [ ] Toolbar layout intact at all desktop widths.
- [ ] `aria-pressed` on the toggle button updates correctly (screen-reader test).

**Edge:**
- [ ] Long `:` input does not break statusbar layout.
- [ ] localStorage-blocked environments (Safari private) do not throw; toggle still works for the session.
- [ ] Resizing below 640px while vim is ON hides the UI but does not crash; resizing back restores it.

## Out of scope

- Vim macros, marks, registers beyond what monaco-vim provides by default.
- Custom `.vimrc`-style user configuration.
- Helix bindings or any selection-first model.
- Mobile vim UI.
- Persisting cursor position or undo history across reloads (unrelated to vim).

## Open implementation questions

These are resolved during implementation, not design — listed here for the writing-plans step to address:

- Exact import shape for `monaco-vim` 0.4+ ESM (`initVimMode` vs `VimMode` named exports; how `CodeMirror.Vim.defineEx` is reached). Verify against installed version before writing code.
- Whether `vite-plugin-monaco-editor` requires any additional config to bundle `monaco-vim` (likely not, since `monaco-vim` lazy-imports from `monaco-editor`).
