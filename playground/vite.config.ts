import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import monacoEditorPlugin from "vite-plugin-monaco-editor";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  // Built output goes into docs/play/ at the repo root so it's served at
  // lisette.run/play by GitHub Pages.  The base URL matches that path.
  base: "/play/",
  build: {
    outDir: "../docs/play",
    emptyOutDir: true,
    target: "es2020",
  },
  plugins: [
    (monacoEditorPlugin as unknown as typeof monacoEditorPlugin.default).default(
      {
        languageWorkers: ["editorWorkerService"],
        // Without this override the plugin appends the base path to outDir,
        // producing docs/play/play/monacoeditorwork (double "play").
        customDistPath: (_root, buildOutDir) =>
          path.join(buildOutDir, "monacoeditorwork"),
      }
    ),
  ],
  server: {
    headers: {
      // Enables SharedArrayBuffer in local dev (Monaco can use it).
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  resolve: {
    alias: {
      "monaco-vim": path.resolve(__dirname, "node_modules/monaco-vim/dist/index.mjs"),
    },
  },
  optimizeDeps: {
    exclude: ["monaco-editor", "monaco-vim"],
  },
  worker: {
    format: "es",
  },
});
