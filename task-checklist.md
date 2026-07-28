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

The first session on this branch covered the **backend only** (crate, CLI). This
follow-up session adds the **frontend** (Tauri command, UI page) that first session
explicitly delegated forward. The branch is still left unmerged, for a final quality
review pass per this session's own instructions.

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

## Frontend (`codepack-desktop-ui` — this session)

- [+] F1 — Tauri command `commands/sanitize.rs`: `start_sanitize`/`cancel_sanitize`,
      background-thread pattern mirroring `commands/export`, `sanitize:finished`
      event. No `sanitize:progress` — `codepack_sanitize::run_sterile_copy` reports
      no intermediate progress today, so no such event would ever fire; noted
      honestly rather than faked. Registered in `lib.rs`'s `invoke_handler` (21
      commands total); `capabilities/default.json` unchanged — no new webview
      permission was needed (folder pickers reuse `dialog:allow-open`, already
      granted)
- [+] F2 — `SterileCopyPage.svelte`: source/destination folder pickers, safety-mode
      `Segmented`, run/cancel button, summary `Stat` row, per-file outcome list.
      Own top-level nav entry (`nav.section.tools`, not nested in the wizard or the
      insight group) — `stores/wizard.svelte.ts` gained a `"sterile"` step and
      `STANDALONE_STEPS`. `api/types.ts` + `api/client.ts` additions
      (`SanitizeReport`/`SanitizeFinishedEvent`, `startSanitize`/`cancelSanitize`/
      `onSanitizeFinished`). i18n: `nav.sterile`, `nav.section.tools`, and a
      `sterile.*` block in both `en.ts` and `ru.ts` (reused `security.safeMode.*`
      for the safety-mode labels rather than duplicating them)

## Verification

- [+] `cargo build -p codepack-desktop`, `cargo test -p codepack-desktop` (72
      passed, including a new `commands::sanitize::tests` unit test), `cargo
      clippy -p codepack-desktop --all-targets -- -D warnings`, `cargo fmt --all
      --check` — all green
- [+] `pnpm install --frozen-lockfile` (this session's environment had no
      `node_modules` at first; installed it), `pnpm --filter @codepack/ui
      typecheck` (136 files, 0 errors), `pnpm --filter @codepack/ui lint` (clean),
      `pnpm format` (Prettier clean)
- [+] Independent quality review (`codepack-quality-reviewer`) of the full diff
      against `main`: redaction reuse, safety-mode reuse, I2 overlap guard,
      tree-sitter fail-safe, string/regex-literal survival, formatter-never-fails,
      per-file cancellation, no Tauri permission widening, dependency graph,
      lock-poison handling, scope discipline, and the honesty of every
      self-reported gap were all traced in the actual code, not trusted from the
      prior sessions' reports. One moderate finding (below), one low (a test-count
      typo in this file, now corrected: `codepack-desktop` unit tests are 71, not
      72 — the extra count in the earlier line included a separate end-to-end
      binary)
- [+] **Fixed the moderate finding**: `validate_destination` ran `create_dir_all`
      on the destination *before* checking for source/destination overlap, so a
      rejected call could still leave a stray directory inside the immutable
      source tree (mirrors an identical, pre-existing ordering in
      `codepack-cli`'s own `export --out` guard on `main` — reproduced faithfully,
      not introduced new, but still worth closing here). Fixed by resolving the
      destination to its prospective canonical path lexically (walk up to the
      nearest existing ancestor, canonicalize that, append the not-yet-created
      suffix — which cannot contain symlinks since it doesn't exist yet) *before*
      any directory is created. New test
      `a_rejected_overlapping_destination_is_never_created_inside_the_source`
      asserts nothing was created, not just that the right error came back.
- [+] Full `cargo xtask gate` re-run end-to-end after the fix: fmt, clippy,
      `cargo test --workspace` (every crate, including 39/39 in
      `codepack-sanitize`, 29/29 in `codepack-cli`), `cargo deny check`, frontend
      format/typecheck (136 files, 0 errors)/lint, the `scripts/` test suite (78
      tests), `sync-agents --check` (29.0 KiB, in sync), network isolation — all
      green
- [+] Manual dev run: `pnpm --dir apps/desktop exec tauri dev` failed on this
      pnpm/corepack version with `--dir` (`Cannot read properties of undefined`,
      an environment issue unrelated to this change); running `pnpm exec tauri
      dev` from `apps/desktop` directly (`.ai/project/15-command-reference.md`'s
      documented working directory) built and launched `codepack-desktop.exe`
      cleanly, watching `codepack-sanitize` among the crates for changes, no
      startup error. Could not click through the running window from this tool
      environment (no GUI interaction capability here) — process was inspected
      via the log and process list, then terminated

## Completion

- [+] `docs/architecture/overview.md` updated: the desktop-app row gets the new
      `commands::sanitize` pair and `sanitize:finished`, the frontend row gets the
      new ninth page and its nav placement
- [+] Checklist filled `+`/`-`, final report in Russian per this project's
      documentation-language policy
- [+] Report any languages from the scanner's stack list left uncovered (Batch 2 /
      unsupported), named honestly, not silently dropped — see final report:
      every non-Batch-1 file is reported as `FileOutcome::SkippedUnsupportedLanguage`
      with an honest reason, never dropped
