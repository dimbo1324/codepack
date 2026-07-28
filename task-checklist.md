# Task Checklist

**Task:** Three small/medium follow-ups suggested after the Sterile-copy feature:
1. `codepack completions <shell>` — shell completion generation via `clap_complete`.
2. Kotlin support for the Sterile-copy sanitizer (closing one Batch 2 language from Q24).
3. Drag-and-drop onto the Sterile-copy UI page (the app already supports drag-and-drop
   globally since the 2026-07-27 decision; the new page never wired it up).

**Date:** 2026-07-28
**Branch:** feat/cli-completions-kotlin-dnd

Owner explicitly approved merging to `main` **and pushing to origin** at the end of this
task (unlike the previous task, where publish was not requested).

## 1 — CLI shell completions

- [+] Add `clap_complete` dependency to `codepack-cli` (workspace dep, MIT/Apache-2.0
      like `clap` itself; `cargo deny check` clean)
- [+] `codepack completions <shell>` subcommand (bash/zsh/fish/powershell/elvish),
      writes the generated script to stdout; no `--json` form — a completion script
      is not a report
- [+] Test: generation succeeds for each supported shell without panicking (unit test
      in `commands/completions.rs`) plus an end-to-end CLI test running the real
      binary for all five shells
- [+] `cargo fmt`/`clippy -D warnings`/`cargo test -p codepack-cli` green for this
      slice

## 2 — Kotlin in `codepack-sanitize`

- [+] Verify `tree-sitter-kotlin` crate: current version, license, maintenance —
      confirm before adding (Q24 flagged Batch 2 crates as needing this check).
      Found the crate literally named `tree-sitter-kotlin` (fwcd, last published
      2024-08-03) pins `tree-sitter >= 0.21, < 0.23`, incompatible with this
      workspace's `tree-sitter` 0.26 — a real type incompatibility, not assumed.
      Used `tree-sitter-kotlin-ng` instead (`tree-sitter-grammars` org fork,
      v1.1.0, MIT, published 2025-01-09, depends on the version-independent
      `tree-sitter-language` crate like every Batch 1 grammar) — builds clean
- [+] Add Kotlin to `language.rs`'s supported set (`.kt`/`.kts` extensions)
- [+] Kotlin comment-stripping test (line `//` and block `/* */`, string/literal
      survival, following the existing Batch 1 test pattern)
- [+] Formatter table entry: `ktlint` has a real stdin-in/stdout-out mode
      (`--stdin --format --stdin-path=<name>`, verified against
      `KtlintCommandLine.kt` source) — added with a real-binary-guarded test
      following the gofmt/ruff pattern
- [+] Update `docs/decisions/open-questions.md` Q24 — Kotlin moves from "deferred"
      to "closed", the rest of Batch 2 (Dart/Swift/Assembly/Groovy) stays open
- [+] `cargo fmt -p codepack-sanitize`/`clippy -D warnings`/`cargo test
      -p codepack-sanitize`/`cargo deny check` all green for this slice (did not
      run the full `cargo xtask gate`, since sections 1 and 3 of this checklist
      were out of scope for this task and are mid-flight on this branch)

## 3 — Drag-and-drop on the Sterile-copy page

- [+] Wire the existing app-level drag-and-drop handler
      (`client.ts::onWindowDragDrop`) into `SterileCopyPage.svelte` for the source
      folder field (destination folder stays picker-only — dropping a folder to
      *write into* is a different, riskier action than dropping one to *read from*)
- [+] Only take effect when the Sterile-copy page is the active view, matching the
      existing single-path-if-multiple-dropped rule from the 2026-07-27 decision.
      `App.svelte` mounts `SterileCopyPage` only while `wizard.step === "sterile"`,
      so the page's own `onMount`/cleanup already scopes the listener; `App.svelte`'s
      own drag-drop handler was additionally given a one-line guard (`if (wizard.step
      === "sterile") return;`) so a single drop is never acted on by both handlers at
      once (this scoping did not pre-exist as assumed in the plan — the global
      handler previously fired unconditionally on every page)
- [+] `pnpm --filter @codepack/ui typecheck`/`lint` clean

## Verification

- [+] Full `cargo xtask gate` green after all three slices landed together: fmt,
      clippy `-D warnings`, 56 test binaries (`cargo test --workspace`, none
      failed), `cargo deny check` (advisories/bans/licenses/sources all ok),
      frontend format/typecheck (136 files, 0 errors)/lint, the `scripts/` test
      suite, `sync-agents --check` (29.0 KiB, in sync), network isolation
- [+] `cargo test --workspace` green (covered by the full gate run above)

## Completion

- [+] `docs/architecture/overview.md` updated (`codepack-cli` row for
      `completions`, `codepack-sanitize` row for Kotlin/`ktlint`)
- [+] Checklist filled `+`/`-`, final report in Russian
- [+] Merge to `main` (fast-forward), push to `origin/main`, delete this branch
      locally (never pushed as its own remote branch, so nothing to delete there)
