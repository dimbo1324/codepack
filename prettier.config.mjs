// Prettier owns everything rustfmt does not: the Svelte frontend, and the repository's
// own configuration files. `cargo xtask fmt` runs both, and `.githooks/pre-commit` runs
// them over staged files.
//
// The settings below are chosen to match the code that is already in the tree, so
// adopting Prettier is a formatting decision rather than a rewrite:
//
// - `printWidth: 100` mirrors `max_width = 100` in `rustfmt.toml`. One wrap column for the
//   whole repository means a reviewer never has to remember which half they are reading.
// - Everything else is Prettier's default on purpose. The existing frontend already uses
//   two-space indentation, double quotes, semicolons, and trailing commas, which is what
//   the defaults produce — so there is nothing to configure and nothing to argue about.
//
// Markdown is deliberately **not** formatted; see `.prettierignore` for why.
/** @type {import("prettier").Config} */
export default {
  printWidth: 100,
  plugins: ["prettier-plugin-svelte"],
  overrides: [
    {
      // Without the plugin above and this parser, Prettier has no idea what a `.svelte`
      // file is and refuses the whole run.
      files: "*.svelte",
      options: { parser: "svelte" },
    },
  ],
};
