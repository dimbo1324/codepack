# Task Checklist

**Task:** Stage **S2 — Scanner: tree walking, ignore rules, stack detection
(`codepack-scanner`)** (`ROADMAP.md` §2).
**Date:** 2026-07-23
**Branch:** feat/s2-scanner-walk-ignore-stack

Scope boundary (binding for this task): `build_export_plan()` applies base-ignore +
stack-ignore + `.exportignore`/custom-rule filtering only. No safe-export-mode
filtering (S3) and no diff/incremental filtering (S4).

## Preparation

- [+] Orientation ritual: git status/log, ROADMAP.md §1 (S2 is the first stage without
      a `**Status.**` line), docs/architecture/overview.md, task-checklist.md (stale
      S1 content carried onto this branch — recreated here), docs/decisions/open-questions.md
- [+] Legacy archive extracted to a session-scratchpad temp dir (outside the repo) and
      read directly: `constants.py`, `utils/text_utils.py`, `utils/path_utils.py`,
      `services/stack_detector.py`, `services/export_ignore.py`, `services/export_plan.py`,
      `services/exporter.py` (to see how `build_export_plan`'s `ignored_dirs` argument
      is actually assembled: `config.effective_ignored_dirs() | merged_extra_ignored_dirs(root)`),
      `config.py::effective_ignored_dirs`, and the corresponding legacy test files
      (`test_stack_detector.py`, `test_export_ignore.py`, `test_export_ignore_edge_cases.py`,
      `test_export_plan.py`, `test_export_plan_edge_cases.py`)

## Implementation

- [+] `Cargo.toml`: added `walkdir`, `rayon`, `regex` to `[workspace.dependencies]`
      with justification comments; wired into `crates/codepack-scanner/Cargo.toml`
      via `dep.workspace = true` (plus already-justified `serde`/`serde_json`/`thiserror`
      reused from S1, and `tempfile` as a dev-dependency)
- [+] `error.rs` — `ScannerError` (thiserror) + `Result<T>` alias
- [+] `constants.rs` — `IGNORED_DIR_NAMES` (18), `TEXT_EXTENSIONS` (133, not the
      135 the task prompt claimed — recounted from the archive, see deviations),
      `BINARY_EXTENSIONS` (84, not 89, same correction), `TEXT_FILENAMES_WITHOUT_EXTENSION`
      (15); all four sets diffed byte-for-byte against the extracted archive
- [+] `classify.rs` — `should_consider_text_file`, `looks_binary` ports
- [+] `stack.rs` — `StackInfo`, the 12-stack rule table, `detect_stacks()`,
      `merged_extra_ignored_dirs()`
- [+] `ignore/` — `ExportIgnoreRules`, `ScanOptions` (+ `From<&codepack_core::Config>`),
      `.exportignore` loader (`mod.rs`); hand-rolled `fnmatch.translate`-equivalent
      matcher backed by `regex` (`pattern.rs`); `should_skip_dir`/`should_skip_file`
      with exact legacy precedence, including the always-include-vs-base-ignore
      subtlety (`rules.rs`)
- [+] `walk.rs` — `IgnoredDirMatcher` (base ∪ stack ∪ config-extra, with the one
      documented glob exception for `*.egg-info`), `walk_project()`: walkdir-based,
      `follow_links(false)` + independent `path_is_symlink()` check, prunes top-down,
      checks `CancellationToken` inside the sequential directory loop; rayon
      parallelizes the per-file `stat()` pass after candidates are collected
- [+] `plan/` (split from a single `plan.rs` — see file-size note below) —
      `PlannedFile`, `ExportPlan`, `PlanSummary`, `build_export_plan()` (S2-scoped
      only), `write_export_plan_files()` (JSON via serde field order + Markdown
      renderer)
- [+] `lib.rs` — crate doc stating the S2 scope boundary explicitly, public
      re-exports, under 100 lines

## Verification

- [+] Unit tests colocated in each module (constants counts, classify table-driven
      cases, stack detection per stack + mono-repo, pattern.rs glob-translate edge
      cases, ignore/rules.rs precedence — including the two explicitly required
      always-include tests: one proving it overrides `.exportignore`/custom exclusion,
      a separate one proving it does NOT override base/stack directory pruning)
- [+] `tests/` integration tests: per-stack fixtures (Node, Python with an ignored
      dir containing a file, to exercise base-ignore pruning end-to-end), a mono-repo
      fixture (Node.js + Python, proves the *union* of both stacks' extra ignored
      dirs is applied, not just the primary one's), a symlink-escape test (I7, created
      programmatically — see deviations, not a static fixture directory), four
      `.exportignore` end-to-end tests, four I5 JSON-contract tests asserting the
      literal serialized key sequence (top-level, `PlannedFile`, `RulesReport`,
      `PlanSummary`), plus a self-check that the contract test would actually catch
      a reorder
- [+] `cargo xtask gate`: fmt, clippy `-D warnings`, tests, `cargo deny check`,
      `sync-agents --check`
- [+] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment
- [+] `docs/architecture/overview.md` updated
- [+] `ROADMAP.md`: `**Status.**` line under S2 + §1 table status flipped to "сделан",
      honestly stating the S2 scope boundary and the `walkdir`-not-`ignore`-crate
      deviation from the S2 "Состав" bullet's own wording

## Completion

- [+] Commits: checklist first, then implementation, separated logically
- [+] Reconciled the implementing subagent's worktree branch onto
      `feat/s2-scanner-walk-ignore-stack` via cherry-pick (task-checklist.md conflict
      resolved in favor of the agent's completed version)
- [+] Caught and fixed a real bug before merge: the implementation commit added
      `walkdir`/`rayon`/`regex`/`codepack-core` to `Cargo.lock` but never declared them
      in either `Cargo.toml` — the crate silently failed to build from a clean lockfile.
      Fixed in a follow-up commit; `cargo xtask gate` is now genuinely green (verified
      in the main working directory, not just the agent's worktree)
- [+] Independent review via `codepack-quality-reviewer`: constant sets, 12-stack
      table, and the always-include-vs-base-ignore precedence all verified correct
      against the legacy archive. Two findings fixed: removed a dead `ScannerError::Read`
      variant; recorded the `*.egg-info` glob-matching deviation in
      `docs/decisions/open-questions.md` (was only in code comments/ROADMAP before)
- [+] CI green on all three OSes: run #40
      (https://github.com/dimbo1324/codepack/actions/runs/29984698173),
      `gate (ubuntu-latest)` / `gate (macos-latest)` / `gate (windows-latest)` — все
      `success` на коммите `5d215ab`
- [+] Fast-forward merge into `main` and push to `origin` (explicit owner sign-off)
- [+] Final report to owner (Russian, per language policy)

## Deviations recorded honestly

- `TEXT_EXTENSIONS`/`BINARY_EXTENSIONS` have 133/84 entries in the actual legacy
  archive, not the 135/89 the task prompt stated — recounted directly from
  `docs/__arch__/codepack-main.zip` (line-range `sed`/`grep`, then diffed sorted lists
  byte-for-byte against my Rust arrays; both matched exactly once corrected). All four
  constant sets (including `IGNORED_DIR_NAMES` and `TEXT_FILENAMES_WITHOUT_EXTENSION`)
  verified this way, not just eyeballed.
- `*.egg-info` (Python stack's one glob-shaped extra-ignored-dir entry) is made to
  actually glob-match in `IgnoredDirMatcher`. Legacy's own `should_ignore_dir` only
  does exact-string set membership, so that entry never really prunes anything in the
  original — the task instructions explicitly asked for the *intended* (working)
  behavior here rather than the literal (inert) legacy behavior; documented inline in
  `walk.rs` and in the `ROADMAP.md` Status line as a deliberate deviation, not a silent
  behavior change.
- No `.exportignore`/`safe_export_mode` semantic changed from legacy — only that one
  glob-matching fix and the walkdir-vs-`ignore`-crate substitution (both required by
  the task's own instructions, not opportunistic).
- Symlink-escape coverage exists as a unit test (`walk.rs`) and an integration test
  (`tests/symlink_escape.rs`), both creating the symlink *programmatically* inside a
  `tempfile::tempdir()` rather than as a static fixture under `tests/fixtures/` — a
  checked-in directory symlink is not reliably portable through a `git` checkout
  across OSes and `core.symlinks` settings, so it would risk breaking `git clone` on
  some machines. Both tests gracefully skip (with an `eprintln!`, not a hard failure)
  if the current environment cannot create a directory symlink (e.g. Windows without
  Developer Mode/admin) — on this dev machine symlink creation succeeded and both
  tests actually exercised the assertion, not just the skip path.
- `cargo-deny` needed `[bans] allow-wildcard-paths = true` added to `deny.toml` — the
  first internal path dependency between workspace crates (`codepack-scanner ->
  codepack-core`) tripped the wildcard-version ban. Recorded as a decision in
  `docs/decisions/open-questions.md` (2026-07-23) since it affects every future
  in-workspace path dependency (`engine -> domain crates -> core`), not just this one.
- `generated_at` renders in UTC via a small dependency-free civil-calendar formatter
  (Howard Hinnant's `civil_from_days` algorithm) instead of legacy's local-time
  `datetime.now().isoformat(...)` — no timezone-database crate is in this stage's
  approved dependency list, and this field is cosmetic (nothing parses it back).
- `PlanSummary` omits legacy's `estimated_included_size` (formatted-bytes string —
  `codepack-core` has no byte formatter until S6) and `skipped_dirs_count` (redundant
  with `skipped_dirs.len()`), exactly as the task instructions asked; documented in
  `plan/mod.rs`'s doc comment.
- Markdown rendering (`plan/render.rs`) shows raw byte counts (`"1234 bytes"`) instead
  of legacy's `format_bytes()` (`"1.21 KB"`) for the same reason — no formatter exists
  yet in `codepack-core`; the task instructions explicitly said not to invent one.
- Did not implement `format_export_plan_for_user` or `format_stack_label` — legacy
  helpers that render a plan/stack summary as plain text for a GUI confirmation
  dialog. Neither was named in the task's file-layout spec (only
  `write_export_plan_files` and `detect_stacks`/`merged_extra_ignored_dirs` were), and
  they are UI-facing text, which is out of scope for a UI-agnostic core crate at this
  stage — `primary_stack` was kept since it is a one-line wrapper already exercised by
  a legacy test and costs nothing to keep for a later stage's use.
