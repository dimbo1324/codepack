# Task Checklist

**Task:** Full-precision timestamps everywhere a date is read or shown — hours, minutes
and seconds, especially for Git commits.

**Date:** 2026-08-05
**Branch:** feat/full-precision-timestamps

Owner instruction, 2026-08-05: whenever information carrying a date is taken, the
precise moment must come with it — hours, minutes, seconds. Git commit inspection is
the named case. Owner approved, once green: merge into `main` and push.

## Preparation

- [+] Orientation ritual: git status/log, ROADMAP, overview, previous checklist,
      open questions — the previous task closed cleanly, nothing left open
- [+] Inventory: every date-producing site was traced through
      `codepack-core::time`, which is the single calendar implementation. Already at
      full precision: `manifest.json` and every artifact header, the CLI `history`
      table, file mtimes, all SQLite rows (epoch seconds). Truncating: the two Group C
      Git reports, and the desktop history table

## Implementation — product

- [+] `codepack-core::time`: `format_date` removed. Without a date-only helper in the
      shared module, truncation has to be written out at the call site where a reviewer
      sees it. A test now asserts no formatter drops the time of day
- [+] `21_git_timeline_report.md`: each of the last 30 commits carries
      `YYYY-MM-DD HH:MM:SS UTC` instead of `YYYY-MM-DD`
- [+] `05_git_deep.txt`: the graph log gained the same stamp — found during the sweep,
      it had no time either
- [+] `04_git_report.txt`: the recent-log section had **no** date at all
      (`git log --oneline -5`); every line now carries the committer moment
- [+] Desktop history table: the exact moment is the visible value; relative time
      ("an hour ago") moved to the tooltip

## Implementation — agent rules

- [+] New universal module `.ai/universal/09-time-and-timestamps.md`, `tier: extended`
- [+] Orientation ritual (`13-progress-tracking.md`) and coordination check
      (`07-multi-assistant.md`) now prescribe
      `git log --date=iso-strict --pretty=format:"%h %cd %s"`
- [+] `.claude/` and `.codex/` mirrors: `stage-episode` skill and the stage-planner
      agent, both sides, same task
- [+] `.ai/CHANGELOG.md` entry; `CLAUDE.md` import added; `AGENTS.md` regenerated

## Documentation

- [+] Owner decision recorded in `docs/__arch__/open-questions.md`, including why
      invariant I5 needs no `schema_version` bump: none of the five artifacts I5 names
      is touched, no file name, section, or field changed — one line inside a section
      got richer
- [-] `docs/architecture/overview.md` unchanged, deliberately: no crate, layer, or
      operational job changed shape. `README.md` likewise — it never described history
      rows or Git report internals at this granularity

## Verification

- [+] `cargo xtask fmt` (Rust and, after `pnpm install`, the frontend)
- [+] `cargo xtask gate` — all eight sections green: format, clippy, tests (56 suites),
      cargo-deny, frontend format/typecheck/lint, dev scripts (78 tests), agents sync,
      network isolation
- [+] Self-review of the diff; two doc comments corrected afterwards and re-verified
- [-] No browser verification of the history table: it renders Tauri command output, so
      a bare Vite dev server would show an empty table and prove nothing. Covered by
      `svelte-check` and ESLint instead

## Completion

- [+] Checklist filled with `+`/`-`
- [ ] Merge into `main` fast-forward, push to `origin/main`
- [ ] Final report
- [ ] Shut down the PC (explicit owner instruction)

## Rule debt carried out of this task

`AGENTS.md` now assembles to 29.9 KiB of its 30 KiB budget. The next rule change of any
size will fail `sync-agents` before it fails anything else. Fixing it properly means
moving an existing module to the extended tier, which drops content from the compiled
entry point and therefore needs an owner decision — not something to do as a side effect
here.
