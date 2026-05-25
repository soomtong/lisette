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
