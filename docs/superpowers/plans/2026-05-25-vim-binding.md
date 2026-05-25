# Vim Binding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add toggleable Vim modal editing to the Lisette Monaco playground via the `monaco-vim` library.

**Architecture:** A single `vim.ts` module encapsulates `monaco-vim`. `main.ts` owns the toggle button, `localStorage` persistence, `Ctrl+Alt+V` shortcut, and passes Format/Run/disable action callbacks into `vim.ts`. The editor module (`editor/index.ts`) remains unaware of vim. Vim status line lives directly under the editor; toggle button lives in the toolbar-right.

**Tech Stack:** TypeScript (strict), Vite 6, Monaco Editor 0.52, `monaco-vim` (new dependency).

**Spec:** `docs/superpowers/specs/2026-05-25-vim-binding-design.md`

**Working directory note:** All paths below are relative to the repo root `/Users/dp/Repository/_study/lisette-lang`. The playground subdir is `playground/`. Branch: `vim-binding`.

**Test strategy:** The playground has no automated test suite. Each task that touches behavior ends with **browser verification steps** against `npm run dev` — this is the project's established pattern (per playground/README.md). Each task also ends with a commit using lowercase Conventional Commits format (lefthook enforces `<type>(scope?)!?: <description>` with ≤72 chars).

---

## File Structure

| Path                                | Action  | Responsibility                                           |
| ----------------------------------- | ------- | -------------------------------------------------------- |
| `playground/package.json`           | modify  | Add `monaco-vim` dependency                              |
| `playground/index.html`             | modify  | Add toggle button + `#vim-statusbar` div                 |
| `playground/src/style.css`          | modify  | Toggle button styles, statusbar styles, mobile hiding    |
| `playground/src/editor/vim.ts`      | create  | `setupVim()` wrapping `monaco-vim`, ex command registry  |
| `playground/src/main.ts`            | modify  | Toggle state, localStorage, shortcut, action wiring      |

---

## Task 1: Install monaco-vim and verify its export shape

**Files:**
- Modify: `playground/package.json`
- Modify: `playground/package-lock.json` (auto-updated by npm)

**Why this is task 1:** The exact import shape of `monaco-vim` (named vs default, where the `Vim` singleton hangs off, ESM/CJS) varies between versions. We confirm it once now and reference the verified shape in subsequent tasks.

- [ ] **Step 1: Install the dependency**

```bash
cd playground
npm install monaco-vim
```

Expected: package.json gains `"monaco-vim": "^<version>"` under `dependencies`. No errors.

- [ ] **Step 2: Inspect the installed package's exports**

Run from `playground/`:
```bash
cat node_modules/monaco-vim/package.json | grep -E '"(main|module|exports|types)"'
ls node_modules/monaco-vim/lib 2>/dev/null || ls node_modules/monaco-vim/dist 2>/dev/null
```

Look for the entry file and types. Then read the entry file's top-level exports:
```bash
grep -E '^export' node_modules/monaco-vim/lib/index.js 2>/dev/null || \
  grep -E 'exports\.' node_modules/monaco-vim/lib/cm_adapter.js 2>/dev/null | head -20
```

Confirm one of these export shapes (record which one applies — used in Task 5):
- **Shape A:** `export { initVimMode, VimMode }` where `VimMode.Vim.defineEx(...)` reaches the singleton.
- **Shape B:** `export default initVimMode` plus `export { VimMode }` similar.
- **Shape C:** CommonJS only — requires `import * as MonacoVim from "monaco-vim"` and `MonacoVim.VimMode.Vim`.

If types are not bundled (no `.d.ts`), the project will need a small ambient declaration in Task 4.

- [ ] **Step 3: Quick smoke test — does it import in this Vite setup?**

Create a throwaway file `playground/src/__vim_smoke.ts`:
```ts
import * as MV from "monaco-vim";
console.log("monaco-vim exports:", Object.keys(MV));
```

In `playground/src/main.ts`, add at the top temporarily:
```ts
import "./__vim_smoke.js";
```

Run `npm run dev` from `playground/`, open the browser console, confirm the keys are printed (expect `initVimMode`, `VimMode`, possibly `default`).

Stop the dev server, remove the temporary import line from `main.ts`, and delete `__vim_smoke.ts`.

- [ ] **Step 4: Commit**

```bash
git add playground/package.json playground/package-lock.json
git commit -m "build: add monaco-vim dependency"
```

---

## Task 2: Add DOM scaffolding for toggle button and status bar

**Files:**
- Modify: `playground/index.html`

- [ ] **Step 1: Add the toggle button to `toolbar-right`**

In `playground/index.html`, replace the `toolbar-right` block (currently containing only `#status-indicator`):

```html
<div class="toolbar-right">
  <button id="btn-vim-toggle" class="btn btn-vim" title="Toggle Vim mode (Ctrl+Alt+V)" aria-pressed="false">
    <span class="vim-toggle-label">Vim</span>
  </button>
  <span id="status-indicator" class="status-idle">Ready</span>
</div>
```

- [ ] **Step 2: Add the vim status bar inside `editor-pane`**

Find the `editor-pane` div in `playground/index.html`:
```html
<div id="editor-pane">
  <div id="editor-container"></div>
</div>
```

Replace with:
```html
<div id="editor-pane">
  <div id="editor-container"></div>
  <div id="vim-statusbar" hidden></div>
</div>
```

- [ ] **Step 3: Browser verification**

Run `npm run dev` from `playground/`, open `http://localhost:5173`:
- Toggle button "Vim" is visible at the right of the toolbar, next to `Ready` status.
- Clicking the button does nothing yet (no handler).
- `#vim-statusbar` is in DOM but hidden (verify via DevTools: it exists but has the `hidden` attribute).
- Editor still renders normally.

- [ ] **Step 4: Commit**

```bash
git add playground/index.html
git commit -m "feat(playground): add vim toggle button and statusbar scaffolding"
```

---

## Task 3: Style the toggle button and status bar

**Files:**
- Modify: `playground/src/style.css`

- [ ] **Step 1: Add `.btn-vim` style next to the existing `.btn-secondary` block**

In `playground/src/style.css`, after the `.btn-secondary:hover` rule (around line 162), add:

```css
.btn-vim {
  background: var(--bg-elevated);
  color: var(--text-secondary);
  padding: 4px 10px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.4px;
  text-transform: uppercase;
}

.btn-vim:hover {
  border-color: var(--text-muted);
  color: var(--text-primary);
}

.btn-vim[aria-pressed="true"] {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
```

- [ ] **Step 2: Add vim status bar styles**

After the existing `#editor-container` rule (around line 199-202), add:

```css
#vim-statusbar {
  flex-shrink: 0;
  padding: 4px 12px;
  background: var(--bg-surface);
  border-top: 1px solid var(--border);
  font-family: "Fira Code", "Cascadia Code", "JetBrains Mono", ui-monospace, monospace;
  font-size: 12px;
  color: var(--text-primary);
  min-height: 22px;
  display: flex;
  align-items: center;
  gap: 8px;
}

#vim-statusbar[hidden] {
  display: none;
}

#vim-statusbar input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: inherit;
  font: inherit;
}
```

- [ ] **Step 3: Hide both the toggle and statusbar on mobile**

Inside the existing `@media (max-width: 640px) { ... }` block, at the very end (before the closing `}` of the media query), add:

```css
  /* Vim mode is a desktop-keyboard feature */
  #btn-vim-toggle {
    display: none;
  }

  #vim-statusbar {
    display: none !important;
  }
```

- [ ] **Step 4: Browser verification**

Run `npm run dev`:
- Toggle button now has compact uppercase "VIM" styling.
- Resize browser to ≤ 640px: toggle button disappears, mobile layout still intact.
- Resize back: toggle button reappears.
- Manually set `hidden=false` on `#vim-statusbar` in DevTools: the status bar shows up below the editor with monospace font, theme-matching background, and 22px height. Re-add `hidden`: it disappears.

- [ ] **Step 5: Commit**

```bash
git add playground/src/style.css
git commit -m "feat(playground): style vim toggle button and statusbar"
```

---

## Task 4: Create vim.ts module skeleton with types

**Files:**
- Create: `playground/src/editor/vim.ts`

- [ ] **Step 1: If Task 1 found that `monaco-vim` ships no `.d.ts`, add ambient declarations**

If types are missing, create `playground/src/editor/monaco-vim.d.ts`:

```ts
declare module "monaco-vim" {
  import type * as Monaco from "monaco-editor";

  export function initVimMode(
    editor: Monaco.editor.IStandaloneCodeEditor,
    statusBar: HTMLElement | null,
  ): { dispose(): void };

  export const VimMode: {
    Vim: {
      defineEx(
        name: string,
        shortName: string,
        handler: (cm: unknown, params: { args?: string[]; argString?: string }) => void,
      ): void;
    };
  };
}
```

(Skip this step if Task 1 confirmed types ship with the package.)

- [ ] **Step 2: Create the vim.ts skeleton**

Create `playground/src/editor/vim.ts`:

```ts
import type * as Monaco from "monaco-editor";

export interface VimActions {
  format: () => void | Promise<void>;
  run:    () => void | Promise<void>;
  disable: () => void;
}

export interface VimHandle {
  dispose: () => void;
}

let exCommandsRegistered = false;

export async function setupVim(
  _editor: Monaco.editor.IStandaloneCodeEditor,
  _statusBar: HTMLElement,
  _actions: VimActions,
): Promise<VimHandle> {
  throw new Error("setupVim: not yet implemented");
}
```

The `_`-prefixed parameter names satisfy `noUnusedParameters` in tsconfig until Task 5 fills the body. `exCommandsRegistered` is the module-scope guard described in the spec (defineEx persists across instances).

- [ ] **Step 3: Verify TypeScript compiles**

From `playground/`:
```bash
npx tsc -b --noEmit
```

Expected: no errors. (The function throws at runtime but the type check passes.)

- [ ] **Step 4: Commit**

```bash
git add playground/src/editor/vim.ts playground/src/editor/monaco-vim.d.ts 2>/dev/null || git add playground/src/editor/vim.ts
git commit -m "feat(playground): scaffold vim.ts module"
```

---

## Task 5: Implement setupVim — initVimMode + dispose

**Files:**
- Modify: `playground/src/editor/vim.ts`

- [ ] **Step 1: Replace the throwing stub with the real implementation**

Replace the entire body of `setupVim` in `playground/src/editor/vim.ts` (keep the imports and interfaces). Use the export shape confirmed in Task 1 — the example below uses Shape A (`import { initVimMode } from "monaco-vim"`). Adjust the import if Task 1 found a different shape.

```ts
import type * as Monaco from "monaco-editor";
import { initVimMode } from "monaco-vim";

export interface VimActions {
  format: () => void | Promise<void>;
  run:    () => void | Promise<void>;
  disable: () => void;
}

export interface VimHandle {
  dispose: () => void;
}

let exCommandsRegistered = false;

export async function setupVim(
  editor: Monaco.editor.IStandaloneCodeEditor,
  statusBar: HTMLElement,
  actions: VimActions,
): Promise<VimHandle> {
  const vim = initVimMode(editor, statusBar);

  // Ex command registration happens in Task 6. Reference `actions` and the
  // guard here so the linter does not flag them as unused.
  void actions;
  void exCommandsRegistered;

  return {
    dispose: () => {
      vim.dispose();
    },
  };
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd playground && npx tsc -b --noEmit
```

Expected: no errors.

- [ ] **Step 3: Manual import-only smoke test**

In a separate terminal: `cd playground && npm run dev`. The dev server should start without errors. No behavior change yet (main.ts is not calling setupVim yet).

- [ ] **Step 4: Commit**

```bash
git add playground/src/editor/vim.ts
git commit -m "feat(playground): wire monaco-vim init and dispose"
```

---

## Task 6: Register ex commands in vim.ts

**Files:**
- Modify: `playground/src/editor/vim.ts`

- [ ] **Step 1: Add the ex command registry function**

In `playground/src/editor/vim.ts`, update the imports and add a `registerExCommands` function. Final file content:

```ts
import type * as Monaco from "monaco-editor";
import { initVimMode, VimMode } from "monaco-vim";

export interface VimActions {
  format: () => void | Promise<void>;
  run:    () => void | Promise<void>;
  disable: () => void;
}

export interface VimHandle {
  dispose: () => void;
}

let exCommandsRegistered = false;

function registerExCommands(actions: VimActions): void {
  // Vim singleton lives on VimMode.Vim — defining ex commands mutates module
  // state, so we guard against re-registering on re-toggle.
  if (exCommandsRegistered) return;
  const Vim = VimMode.Vim;

  Vim.defineEx("write", "w",   () => { void actions.format(); });
  Vim.defineEx("fmt",   "fmt", () => { void actions.format(); });
  Vim.defineEx("run",   "r",   () => { void actions.run(); });
  Vim.defineEx("vim",   "vim", () => { actions.disable(); });

  exCommandsRegistered = true;
}

export async function setupVim(
  editor: Monaco.editor.IStandaloneCodeEditor,
  statusBar: HTMLElement,
  actions: VimActions,
): Promise<VimHandle> {
  registerExCommands(actions);
  const vim = initVimMode(editor, statusBar);
  return {
    dispose: () => vim.dispose(),
  };
}
```

**Note on the closure:** the `actions` callbacks captured by `defineEx` on the *first* call persist for the lifetime of the page. On re-toggle (creating a new vim instance), we keep the original closure — this is fine because `main.ts` exposes stable function references (the top-level `run`, `formatAction`, `toggleVim` functions don't get re-created per toggle).

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd playground && npx tsc -b --noEmit
```

Expected: no errors. If Task 4 added the ambient declaration, ensure `VimMode.Vim.defineEx` signature matches the actual handler call sites (3 args).

- [ ] **Step 3: Commit**

```bash
git add playground/src/editor/vim.ts
git commit -m "feat(playground): register vim ex commands w/fmt/run/r/vim"
```

---

## Task 7: Wire toggle state and localStorage in main.ts

**Files:**
- Modify: `playground/src/main.ts`

- [ ] **Step 1: Add imports and constants near the top of main.ts**

In `playground/src/main.ts`, add to the imports block (after the existing imports, around line 6):

```ts
import { setupVim, type VimActions, type VimHandle } from "./editor/vim.js";
```

Add the localStorage key constant near the top, after imports:

```ts
const VIM_STORAGE_KEY = "lisette-playground:vim-mode";

function readVimEnabled(): boolean {
  try {
    return localStorage.getItem(VIM_STORAGE_KEY) === "on";
  } catch {
    return false;
  }
}

function writeVimEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(VIM_STORAGE_KEY, enabled ? "on" : "off");
  } catch {
    // Safari private mode etc — no-op, in-memory state still works.
  }
}
```

- [ ] **Step 2: Add the DOM refs for the toggle button and statusbar**

In the existing `// ─── DOM refs ───` block (around lines 42-54), add:

```ts
const btnVimToggle   = document.getElementById("btn-vim-toggle") as HTMLButtonElement;
const vimStatusBar   = document.getElementById("vim-statusbar") as HTMLElement;
```

- [ ] **Step 3: Add toggle state and `toggleVim()` function inside `main()`**

Inside the `async function main()` body, after `setupEditors(...)` returns (around line 147), add:

```ts
let vimHandle: VimHandle | null = null;

// Defined later via closure capture so it can reach `bridge`, run, etc.
// We forward-declare actions and toggleVim here.
const vimActions: VimActions = {
  format: async () => { btnFormat.click(); },
  run:    async () => { btnRun.click(); },
  disable: () => { toggleVim(false); },
};

async function toggleVim(next: boolean): Promise<void> {
  if (next && !vimHandle) {
    vimStatusBar.hidden = false;
    vimHandle = await setupVim(editorResult.mainEditor, vimStatusBar, vimActions);
    btnVimToggle.setAttribute("aria-pressed", "true");
    writeVimEnabled(true);
  } else if (!next && vimHandle) {
    vimHandle.dispose();
    vimHandle = null;
    vimStatusBar.hidden = true;
    btnVimToggle.setAttribute("aria-pressed", "false");
    writeVimEnabled(false);
  }
  editorResult.mainEditor.focus();
}
```

**Why route format/run through `btnFormat.click()` / `btnRun.click()`:** the existing format/run logic is wrapped inside event listeners (lines 195-243 of main.ts). Calling `.click()` reuses those flows verbatim — no duplication, no state divergence. The downside is one synthetic event per ex command, which is negligible.

- [ ] **Step 4: Apply initial state on load**

Immediately after the `vimActions` / `toggleVim` block, add:

```ts
if (readVimEnabled()) {
  await toggleVim(true);
}
```

- [ ] **Step 5: Verify TypeScript compiles**

```bash
cd playground && npx tsc -b --noEmit
```

Expected: no errors.

- [ ] **Step 6: Browser verification (button still has no click handler — Task 8)**

`npm run dev`:
- First visit (clear localStorage): page loads, vim OFF, no statusbar, editing normal.
- DevTools console: `localStorage.setItem("lisette-playground:vim-mode", "on")` and reload — vim mode auto-activates, statusbar visible, `-- NORMAL --` displayed, h/j/k/l navigates.
- DevTools console: `localStorage.setItem("lisette-playground:vim-mode", "off")` and reload — vim mode OFF.

- [ ] **Step 7: Commit**

```bash
git add playground/src/main.ts
git commit -m "feat(playground): persist vim mode state via localStorage"
```

---

## Task 8: Wire toggle button click and Ctrl+Alt+V shortcut

**Files:**
- Modify: `playground/src/main.ts`

- [ ] **Step 1: Add the button click handler**

In `playground/src/main.ts`, inside `main()`, after the initial-state block from Task 7, add:

```ts
btnVimToggle.addEventListener("click", () => {
  void toggleVim(!vimHandle);
});
```

- [ ] **Step 2: Add the Ctrl+Alt+V keyboard shortcut**

Find the existing `Ctrl+Enter` shortcut handler (around line 321):

```ts
document.addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
    e.preventDefault();
    run();
  }
});
```

Extend the same listener (or add a second one — separate listener is cleaner):

```ts
document.addEventListener("keydown", (e) => {
  if (e.ctrlKey && e.altKey && (e.key === "v" || e.key === "V")) {
    e.preventDefault();
    void toggleVim(!vimHandle);
  }
});
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd playground && npx tsc -b --noEmit
```

Expected: no errors.

- [ ] **Step 4: Browser verification**

`npm run dev`:
- Click toggle button: vim mode turns ON, button gains accent color, `aria-pressed="true"` (check via DevTools), `-- NORMAL --` shown.
- Click again: vim mode turns OFF, button reverts to default style, statusbar hidden.
- Press `Ctrl+Alt+V`: toggles identically.
- Reload after toggling ON: state persists.

- [ ] **Step 5: Commit**

```bash
git add playground/src/main.ts
git commit -m "feat(playground): wire vim toggle button and ctrl+alt+v shortcut"
```

---

## Task 9: Full manual verification & status indicator polish

**Files:**
- Modify: `playground/src/main.ts` (optional polish)

This task runs the full verification checklist from the spec and fixes any issues found.

- [ ] **Step 1: Run the spec's verification checklist**

Start `npm run dev` and walk through every item below. Record any failures.

**Basic toggle:**
- [ ] First load (with localStorage cleared): vim OFF, button unpressed, statusbar hidden.
- [ ] Click toggle → vim ON, statusbar shows `-- NORMAL --`.
- [ ] `h`/`j`/`k`/`l` navigate; `i` enters insert; `Esc` returns to normal; `dd` deletes a line; `u` undoes.
- [ ] Click toggle → vim OFF, editing reverts to plain Monaco.
- [ ] Reload → state preserved.
- [ ] `Ctrl+Alt+V` toggles identically.

**Ex commands:**
- [ ] In normal mode, type `:w` → file formats, status indicator updates.
- [ ] `:fmt` → formats.
- [ ] `:run` → executes (`Ctrl+Enter` equivalent).
- [ ] `:r` → executes.
- [ ] `:vim` → turns vim mode OFF.

**Regression (vim OFF):**
- [ ] Run, Format, Check, Share buttons work.
- [ ] Diagnostics tab clickable items jump to position.
- [ ] Output drawer toggle works on mobile.
- [ ] Pane resizer drags.

**Regression (vim ON):**
- [ ] In insert mode, `Ctrl+Enter` still runs the code (this is the most important regression test — bubbling to document listener).
- [ ] In normal mode, `Alt+Shift+F` formats.
- [ ] Toolbar buttons still work via mouse.

**Mobile (resize browser to ≤ 640px):**
- [ ] Toggle button hidden.
- [ ] If vim was ON before resize: statusbar also hidden (instance still alive, no errors).
- [ ] Resizing back: toggle button and statusbar UI return.

**Theme:**
- [ ] Toggle OS dark/light: statusbar background and toggle button colors track the theme.

**Edge:**
- [ ] In DevTools, run `localStorage.clear()` then reload: vim defaults to OFF.
- [ ] In Safari Private mode (or simulate by overriding `localStorage.setItem` to throw): toggling still works in-memory.

- [ ] **Step 2: (Optional polish) Surface ex-command actions in the status indicator**

If you want explicit feedback that `:w` triggered Format (per the spec): after the `btnFormat.click()` / `btnRun.click()` calls inside `vimActions`, the existing format/run handlers already call `setStatus(...)`. No additional code needed — the status indicator updates naturally.

If any verification step failed, fix it now. Common likely fixes:
- If ex-command output looks weird in the statusbar: check the `#vim-statusbar input` CSS rule from Task 3.
- If `Ctrl+Enter` in insert mode does **not** run code: the document keydown listener for Ctrl+Enter may need to be re-bound (`{ capture: true }`) so it fires before monaco-vim handles the event.
- If toggle from vim ON → OFF leaves stale visual artifacts: ensure `vimStatusBar.hidden = true` runs after `dispose()` (already correct in Task 7).

- [ ] **Step 3: Commit any fixes**

If fixes were needed:
```bash
git add playground/src/main.ts playground/src/style.css
git commit -m "fix(playground): <short description of fix>"
```

If no fixes were needed, skip this step.

---

## Task 10: Production build smoke check

**Files:** none

- [ ] **Step 1: Production build runs cleanly**

```bash
cd playground
npm run build
```

Expected: build succeeds, no TypeScript errors, no Vite warnings about `monaco-vim`. The output `docs/play/` is rebuilt.

- [ ] **Step 2: Preview the production build**

```bash
npm run preview
```

Open the preview URL. Quickly verify:
- Vim toggle button visible.
- Click toggle → vim mode activates.
- `:w` formats.
- `Ctrl+Enter` runs.

- [ ] **Step 3: Do NOT commit `docs/play/`**

`docs/play/` is rebuilt by `just rebuild-playground` at release time per playground/README.md — committing it from this branch would cause merge churn. Verify it is not staged:

```bash
git status playground/ docs/play/
```

If `docs/play/` shows up as modified, leave it unstaged. (It will be rebuilt and committed during the release process.)

- [ ] **Step 4: Final commit if any docs/source remain**

If the spec mentions updating `playground/README.md` to document the vim toggle, add a brief note:

In `playground/README.md`, under the `## Features` list, add a bullet:

```
- **Vim mode** — toggleable modal editing via `monaco-vim`. Click "VIM" in the toolbar or press `Ctrl+Alt+V`. Ex commands: `:w`/`:fmt` → Format, `:run`/`:r` → Run, `:vim` → toggle off.
```

Then commit:

```bash
git add playground/README.md
git commit -m "docs(playground): document vim mode in playground readme"
```

---

## Notes for the executing engineer

- **Lefthook commit-msg hook** enforces lowercase Conventional Commits with ≤72 chars. Subjects above use this format.
- **No automated tests exist for the playground.** Browser verification is the project's chosen pattern — do not invent a test framework as part of this work.
- **`monaco-vim` is a single-instance singleton at the module level.** Re-toggling creates new instances but re-uses the same `VimMode.Vim` namespace. Do not call `defineEx` more than once across the page lifetime; the `exCommandsRegistered` guard in `vim.ts` handles this.
- **The `vim.ts` `import { ... } from "monaco-vim"` syntax is provisional** until Task 1 verifies the actual export shape. Adjust Task 5/6 imports based on Task 1's findings before proceeding.
- **Branch is `vim-binding`** — already created and contains the spec commit. Do not switch branches or rebase during implementation.
