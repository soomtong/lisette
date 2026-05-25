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
