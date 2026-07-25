// ESLint flat config (v9+).
//
// Kept close to the recommended sets rather than curated rule by rule: this package is a
// thin view layer, and a large hand-tuned rule list would be more configuration to
// maintain than code to check. `svelte-check` already covers types and Svelte
// correctness, so ESLint's job here is the things a type checker does not see.
import js from "@eslint/js";
import svelte from "eslint-plugin-svelte";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    // Build output and dependencies are not ours to lint.
    ignores: ["dist/", "node_modules/", ".svelte-kit/"],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs.recommended,
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.es2024 },
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts"],
    languageOptions: {
      parserOptions: {
        // The Svelte parser needs the TS parser for `<script lang="ts">` blocks.
        parser: tseslint.parser,
      },
    },
  },
  {
    // Config files run in Node, not the browser.
    files: ["*.config.js", "*.config.ts"],
    languageOptions: {
      globals: { ...globals.node },
    },
  },
);
