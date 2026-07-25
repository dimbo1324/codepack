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
    // `true` rather than a named minifier: Vite 8 bundles Oxc and no longer ships
    // esbuild, so naming `"esbuild"` (as the create-tauri-app template did, written
    // against Vite 5/6) sends the build down a deprecated path that fails outright.
    // Letting Vite pick its own default keeps this working across the next change too.
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}));
