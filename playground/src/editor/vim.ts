import type * as Monaco from "monaco-editor";

export interface VimActions {
  format: () => void | Promise<void>;
  run:    () => void | Promise<void>;
  disable: () => void;
}

export interface VimHandle {
  dispose: () => void;
}

// Guard: defineEx calls persist across editor instances, so register only once.
// eslint-disable-next-line import/no-mutable-exports
export let exCommandsRegistered = false;

export async function setupVim(
  _editor: Monaco.editor.IStandaloneCodeEditor,
  _statusBar: HTMLElement,
  _actions: VimActions,
): Promise<VimHandle> {
  throw new Error("setupVim: not yet implemented");
}
