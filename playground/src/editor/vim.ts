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
