# Task Checklist

**Task:** Stage **S2 — Scanner: tree walking, ignore rules, stack detection**
(`codepack-scanner`) (`ROADMAP.md` §2).
**Date:** 2026-07-23
**Branch:** feat/s2-scanner-walk-ignore-stack

## Preparation

- [+] Orientation ritual: git status/log, ROADMAP.md §1 (S2 is the first stage without
      a `**Status.**` line), docs/architecture/overview.md, task-checklist.md,
      docs/decisions/open-questions.md — no open items blocking S2
- [+] Delegated stage planning to `codepack-stage-planner`: legacy archive extracted to
      a session-scratchpad temp dir and read (`constants.py`, `stack_detector.py`,
      `export_ignore.py`, `export_plan.py`, `path_utils.py`, `text_utils.py`, and the
      matching legacy test files as behavioral oracles)
- [+] Resolved scope boundary: `codepack-scanner` depends only on `codepack-core`. No
      safe-mode filtering (S3, `codepack-security`) or diff/incremental filtering (S4,
      `codepack-diff`) inside `build_export_plan()`. Golden/oracle tests use the
      legacy-equivalent baseline (`safe_export_mode=full`, `diff_export_mode=all`,
      `incremental_export_enabled=false`) where those filters are no-ops.
- [+] Resolved: no real legacy Python process is run to produce goldens (would need an
      isolated venv and doesn't fit this session's scope/time). Behavioral oracle is
      the legacy pytest source itself — test bodies and their literal assertions are
      ported 1:1 into Rust tests, the same approach S1 used for config-edge-case parity.
- [+] Resolved pattern-matching dependency spike: hand-roll a `fnmatch.translate`-style
      port backed by `regex`, not `globset` — avoids unverified semantic assumptions
      about `globset`'s `literal_separator` handling matching Python's `fnmatch`
      exactly; `regex` is also an anticipated S3 dependency (BLUEPRINT §C.3).

## Implementation

- [ ] `crates/codepack-scanner/Cargo.toml`: add `walkdir`, `rayon`, `regex`,
      `codepack-core` (path dep) to `[workspace.dependencies]` / this crate
- [ ] `src/constants.rs` — `IGNORED_DIR_NAMES` (18), `TEXT_EXTENSIONS` (135),
      `BINARY_EXTENSIONS` (89), `TEXT_FILENAMES_WITHOUT_EXTENSION` (15), ported verbatim
- [ ] `src/classify.rs` — `should_consider_text_file` + `looks_binary` ports
- [ ] `src/stack.rs` — 12-stack rule table, `detect_stacks()` (all matches, sorted by
      marker count), `merged_extra_ignored_dirs()` (union over all matched stacks)
- [ ] `src/ignore/` — `ExportIgnoreRules`, `ScanOptions` (+ `From<&codepack_core::Config>`
      adapter), pattern matcher (`fnmatch`-equivalent), `should_skip_dir`/
      `should_skip_file` with exact legacy precedence (always-include checked before
      any exclusion; base `IGNORED_DIR_NAMES`/stack dirs are pruned before `.exportignore`
      is ever consulted, so always-include cannot rescue a base-ignored subtree)
- [ ] `src/walk.rs` — `walkdir`-based traversal, top-down pruning, never follows
      symlinks, `CancellationToken` checked inside the loop, `rayon` for the per-file
      classification pass (not the pruning walk itself)
- [ ] `src/plan.rs` — `PlannedFile`, `ExportPlan`, `build_export_plan()` (S2-scoped),
      `write_export_plan_files()` (JSON/MD renderer matching legacy field names/order)
- [ ] `src/error.rs` — `ScannerError` (thiserror) + `Result` alias
- [ ] `src/lib.rs` — public re-exports, crate doc noting the S2 scope boundary
- [ ] Fixtures: one tiny synthetic project per stack (12) + 1 mono-repo (2+ stacks) +
      1 symlink-escape fixture, under `tests/fixtures/`

## Verification

- [ ] Constant-set tests (cardinality + curated member/non-member samples)
- [ ] Per-stack detection tests (12) + mono-repo union test + primary-stack ordering
- [ ] `.exportignore` rule tests ported 1:1 from `test_export_ignore*.py`
- [ ] Property test: always-include beats custom `.exportignore`/config exclusion;
      separate test proves base `IGNORED_DIR_NAMES`/stack dirs are NOT overridable
- [ ] Symlinked directory is never descended into
- [ ] `build_export_plan()` oracle tests ported from `test_export_plan*.py` (S2-scoped
      baseline settings) — field names/order match the legacy JSON contract
- [ ] `write_export_plan_files()` Markdown output matches the legacy template structure
- [ ] Classification unit tests (`should_consider_text_file`, `looks_binary` incl. the
      30% boundary — confirm `>` not `>=`)
- [ ] `cargo xtask gate` green locally (fmt, clippy `-D warnings`, tests, `cargo deny
      check`, `sync-agents --check`)
- [ ] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment

## Completion

- [ ] `docs/architecture/overview.md` updated
- [ ] `ROADMAP.md`: `**Status.**` line under S2 (Russian) + §1 table status updated,
      including the honest scoping note (safety/diff filtering deferred to S3/S4/S9)
- [ ] CI green on all three OSes (confirm after push)
- [ ] Commits: checklist first, then implementation, separated logically
- [ ] Fast-forward merge into `main` (after explicit owner sign-off, per workflow)
- [ ] Final report to owner (Russian, per language policy)

---

## Next task

Stage **S3 — Безопасность (`codepack-security`)** (`ROADMAP.md` §2) — ⭐ core value
of the whole project. Start with the orientation ritual from
`.ai/project/13-progress-tracking.md`.
