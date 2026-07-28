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

- [ ] Verify `tree-sitter-kotlin` crate: current version, license, maintenance —
      confirm before adding (Q24 flagged Batch 2 crates as needing this check)
- [ ] Add Kotlin to `language.rs`'s supported set (`.kt`/`.kts` extensions)
- [ ] Kotlin comment-stripping test (line `//` and block `/* */`, string/literal
      survival, following the existing Batch 1 test pattern)
- [ ] Formatter table entry if a safe stdin-invocable Kotlin formatter exists
      (`ktlint`); if not, honestly falls back to `StrippedOnlyNoFormatterFound`
      like Java/C#/PHP/Ruby already do
- [ ] Update `docs/decisions/open-questions.md` Q24 — Kotlin moves from "deferred"
      to "closed", the rest of Batch 2 (Dart/Swift/Assembly/Groovy) stays open
- [ ] `cargo xtask gate` green for this slice

## 3 — Drag-and-drop on the Sterile-copy page

- [ ] Wire the existing app-level drag-and-drop handler
      (`client.ts::onWindowDragDrop`) into `SterileCopyPage.svelte` for the source
      folder field (destination folder stays picker-only — dropping a folder to
      *write into* is a different, riskier action than dropping one to *read from*)
- [ ] Only take effect when the Sterile-copy page is the active view, matching the
      existing single-path-if-multiple-dropped rule from the 2026-07-27 decision
- [ ] `pnpm --filter @codepack/ui typecheck`/`lint` clean

## Verification

- [ ] Full `cargo xtask gate` green after all three slices land
- [ ] `cargo test --workspace` green

## Completion

- [ ] `docs/architecture/overview.md` updated where relevant
- [ ] Checklist filled `+`/`-`, final report in Russian
- [ ] Merge to `main` (fast-forward), push to `origin/main`, delete this branch
      locally and (if pushed as its own branch) on the remote
