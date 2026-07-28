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

This session covered the **backend only** (crate, CLI). Frontend (Tauri command, UI
page) is explicitly delegated to a follow-up task per the task instructions — the
branch is left as-is for it, not merged.

## Preparation

- [+] Record owner decision + Q24 (risks: source/dest overlap, network-capable
      formatters, tree-sitter parse-error fallback, grammar version compatibility,
      Batch 2 language license/maturity sign-off) in `docs/decisions/open-questions.md`
      (already done by a prior session before this one started)
- [+] Commit checklist + decision record before main work (already committed as
      `a68794a` before this session)

## Backend (`codepack-sanitize`)

- [+] B1 — crate skeleton: `SterileCopyOptions`/`FileOutcome`/`SterileCopyReport`,
      wired to `codepack-scanner` (file selection) + `codepack-security` (safety
      skip + redaction) + `codepack-core`; Batch 1 tree-sitter comment strippers
      (JS/TS, Python, Go, Rust, Java, C#, PHP, Ruby, C, C++, Shell, Make); no
      formatting yet
- [+] B2 — formatter-detection layer (PATH + PATHEXT resolution, per-language
      invocation table), `FileOutcome::StrippedOnlyNoFormatterFound` on miss,
      never fails the whole run. Working stdin formatters: rustfmt, prettier,
      ruff format → black, gofmt, clang-format, shfmt. Java/C#/PHP/Ruby have no
      table entry at all (no safe stdin-only mode found) — always
      `StrippedOnlyNoFormatterFound`, named honestly rather than guessed at.
- [+] B3 — `codepack sanitize --source <path> --out <path> [--safe-mode <mode>]`
      CLI subcommand, human and `--json` output, exit code `Incomplete` when any
      file errored
- [+] Source/destination overlap guard (invariant I2 style check), enforced on
      canonicalized paths before any file is touched
- [+] Unit tests per language fixture; redaction-not-bypassed test; cancellation
      test (both "cancelled before the plan is built" and "cancelled inside the
      per-file loop")

## Frontend (delegated to codepack-desktop-ui — NOT done in this task)

- [-] F1 — Tauri command `commands/sanitize.rs` + progress event — not started;
      explicitly out of scope for this task per its own instructions
- [-] F2 — `SterileCopyPage.svelte` + navigation entry + `api/types.ts`
      additions — not started, same reason

## Verification

- [+] `cargo xtask gate` green (full gate, including `cargo deny`, `sync-agents
      --check`, the `scripts/` suite; frontend steps skipped with a notice —
      `apps/desktop/ui/node_modules` is absent in this environment, which the
      gate itself treats as acceptable outside `CI`)
- [+] `cargo test --workspace` green: 38 new `codepack-sanitize` tests, 29
      `codepack-cli` end-to-end tests (3 new for `sanitize`) — no existing test
      anywhere in the workspace was weakened or deleted
- [-] `svelte-check` clean for the new page/command — no page/command exists yet
      (frontend out of scope)
- [-] Manual run in the dev app — no UI entry point exists yet; the CLI was
      exercised manually instead (`codepack sanitize --source ... --out ...`
      against a real mixed-language scratch folder, confirmed stripped +
      formatted output and a written `STERILE_COPY_REPORT.md/.json`)

## Completion

- [+] `docs/architecture/overview.md` updated with the new crate and the
      `codepack-cli` `sanitize` subcommand
- [+] Checklist filled `+`/`-`, final report in Russian per this project's
      documentation-language policy
- [+] Report any languages from the scanner's stack list left uncovered (Batch 2 /
      unsupported), named honestly, not silently dropped — see final report:
      every non-Batch-1 file is reported as `FileOutcome::SkippedUnsupportedLanguage`
      with an honest reason, never dropped
