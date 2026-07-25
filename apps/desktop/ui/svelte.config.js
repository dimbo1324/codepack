// Svelte's own configuration, separate from Vite's.
//
// Present mainly so `vite-plugin-svelte` stops reporting that it fell back to defaults on
// every build — a warning that trains people to ignore build output. The one real setting
// is the preprocessor, which is what lets `<script lang="ts">` and `.svelte.ts` rune
// modules compile.
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    // Runes explicitly rather than by inference. Every component in this package uses
    // them, and the inferred mode depends on whether a file happens to contain a rune —
    // so a component that used none would silently compile in legacy mode.
    runes: true,
  },
};
