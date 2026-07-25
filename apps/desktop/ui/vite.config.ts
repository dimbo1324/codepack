import { fileURLToPath, URL } from "node:url";

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

// Tauri-specific settings, following the upstream `create-tauri-app` template:
// fixed dev-server port (the Rust shell's `tauri.conf.json` points at it by
// number), no dep pre-bundling of the Tauri API packages (they use Node-style
// resolution that Vite's optimizer otherwise mishandles), and preserved
// console/debugger statements in dev builds so `RUST_LOG`-style debugging works
// through the webview's own devtools.
export default defineConfig(async () => ({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL("./src/lib", import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}));
