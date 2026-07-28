# Task Checklist

**Task:** Add the "Sterile copy" feature — a standalone action that takes a source
project folder and produces a separate destination folder with all comments stripped
(via tree-sitter, per language) and the result auto-formatted with whatever external
formatter is found on PATH. New backend crate `codepack-sanitize`, new CLI subcommand,
new Tauri command, new UI page.

**Date:** 2026-07-28
**Branch:** feat/sterile-project-copy

This is new product scope not present in `BLUEPRINT.md`/`ROADMAP.md`, explicitly
authorized by the owner in the current conversation. Scoped by `codepack-stage-planner`
before implementation; owner decisions on method (tree-sitter, PATH-based formatters,
scanner's full stack list, standalone UI action) are recorded in
`docs/decisions/open-questions.md` (Q24) before any code is written.

## Preparation

- [ ] Record owner decision + Q24 (risks: source/dest overlap, network-capable
      formatters, tree-sitter parse-error fallback, grammar version compatibility,
      Batch 2 language license/maturity sign-off) in `docs/decisions/open-questions.md`
- [ ] Commit checklist + decision record before main work

## Backend (`codepack-sanitize`, delegated to codepack-core-engine)

- [ ] B1 — crate skeleton: `SterileCopyOptions`/`FileOutcome`/`SterileCopyReport`,
      wired to `codepack-scanner` (file selection) + `codepack-security` (safety
      skip + redaction) + `codepack-core`; Batch 1 tree-sitter comment strippers
      (JS/TS, Python, Go, Rust, Java, C#, PHP, Ruby, C, C++, Shell, Make); no
      formatting yet
- [ ] B2 — formatter-detection layer (PATH + PATHEXT resolution, per-language
      invocation table), `FileOutcome::StrippedOnlyNoFormatterFound` on miss,
      never fails the whole run
- [ ] B3 — `codepack sanitize --source <path> --out <path>` CLI subcommand
- [ ] Source/destination overlap guard (invariant I2 style check)
- [ ] Unit tests per language fixture; redaction-not-bypassed test; cancellation test

## Frontend (delegated to codepack-desktop-ui)

- [ ] F1 — Tauri command `commands/sanitize.rs` + progress event, independent of UI
- [ ] F2 — `SterileCopyPage.svelte` + navigation entry + `api/types.ts` additions

## Verification

- [ ] `cargo xtask gate` green
- [ ] `cargo test --workspace` green, including new `codepack-sanitize` tests
- [ ] `svelte-check` clean for the new page/command
- [ ] Manual run in the dev app: pick a real mixed-language folder, verify output
      folder has comments stripped and formatting applied where a formatter exists

## Completion

- [ ] `docs/architecture/overview.md` updated with the new crate
- [ ] Checklist filled `+`/`-`, final report in Russian per this project's
      documentation-language policy
- [ ] Report any languages from the scanner's stack list left uncovered (Batch 2 /
      unsupported), named honestly, not silently dropped
