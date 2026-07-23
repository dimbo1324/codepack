# Task Checklist

**Task:** Stage **S1 — Domain types and configuration (`codepack-core`)** (`ROADMAP.md` §2).
**Date:** 2026-07-22
**Branch:** feat/s1-core-domain-types-config

Note: this checklist is written in English. `.ai/project/10-project-map.md` lists
`task-checklist.md` as agent-facing infrastructure (English); the S0 checklist was
written in Russian, which was drift against that same policy line. Correcting it here
rather than silently rewriting the already-merged S0 one.

## Preparation

- [+] Orientation ritual: git status/log, ROADMAP.md §1 (S1 is the first stage without
      a `**Status.**` line), docs/architecture/overview.md, task-checklist.md,
      docs/decisions/open-questions.md — no open items blocking S1
- [+] Delegated stage planning to `codepack-stage-planner`: legacy archive extracted to
      a session-scratchpad temp dir (outside the repo) and read (`config.py`,
      `constants.py`, `models.py`, config-edge-case and AI-preset tests)
- [+] Resolved scope ambiguities from the plan:
      - `BLUEPRINT.md` §A.3's own field table already lists **26** rows (last_root
        through prompt_goals) — the "25" figure is a miscount that exists only in
        `ROADMAP.md` (two places), not in BLUEPRINT's table itself. Fixed the
        `ROADMAP.md` wording as routine doc maintenance; `BLUEPRINT.md` needed no edit.
      - AI presets (5-entry static table, ROADMAP §6 lists S1+S7): included in S1 as
        data-only (`AiPreset` type + table), no application/wiring logic — that is S7.
      - Project-level `.codepack.toml` (BLUEPRINT §B.7, no legacy counterpart): out of
        S1 scope; recorded as an open question (which stage owns it) rather than built
        or silently dropped.
      - `cargo deny` wiring into the gate: `deny.toml` already says "runs once the
        workspace has real dependencies (stage S1 onward)" — S1 is that stage, so wired
        it into `cargo xtask gate` and CI now.

## Implementation

- [+] Added `serde`, `serde_json`, `thiserror`, `crossbeam-channel` to
      `[workspace.dependencies]`; `tempfile` as a dev-dependency; wired into
      `crates/codepack-core/Cargo.toml` via `dep.workspace = true` (`directories` was
      planned here too but dropped — see "Mid-implementation deviations" below)
- [+] `crates/codepack-core/src/error.rs` — `CoreError` (thiserror) + `Result<T>` alias
- [+] `crates/codepack-core/src/types.rs` — `ExportPaths`, `CopyStats`, `TextDumpStats`,
      `RiskPreviewItem`, `RiskPreviewReport` (+ `has_warnings`), `ArchiveBuildResult`
      (+ `primary_result`)
- [+] `crates/codepack-core/src/cancellation.rs` — `CancellationToken`
- [+] `crates/codepack-core/src/progress.rs` — `ProgressEvent`/`LogEvent` +
      crossbeam-channel aliases
- [+] `crates/codepack-core/src/paths.rs` — `AppPaths` resolved by hand from
      environment variables (not `directories`, see below), injectable base dir for
      hermetic tests, legacy settings-file location resolver
- [+] `crates/codepack-core/src/config/` directory module: `mod.rs` (26-field struct,
      declaration order matches legacy), `normalize.rs` (methods on `&Config`, handles
      the `diff_export_mode`/`incremental_export_enabled` coupling), `valid_sets.rs`
      (export profiles / safe modes / diff modes + legacy aliases), `legacy.rs`
      (tolerant load + the one real migration rule), `presets.rs` (5 AI presets, data
      only), `io.rs` (load/save via `paths.rs`)
- [+] `crates/codepack-core/src/lib.rs` — replaced the placeholder, re-exported the
      public surface
- [+] `crates/codepack-core/tests/fixtures/` — 4 legacy-settings fixtures (full,
      missing-flag, unknown-keys, corrupt) + `legacy_migration.rs` integration test
- [+] Wired `cargo deny check` into `cargo xtask gate` and `.github/workflows/ci.yml`

## Verification

- [+] Unit tests for all 26 `Config` fields: valid passthrough + invalid/out-of-range
      fallback, including the diff-mode/incremental coupling, `ui_zoom` clamp/parse
      fallback, the two legacy diff-mode aliases
- [+] Round-trip test (`Config` → JSON → `Config`) + JSON-shape/contract test (26 keys
      present, `schema_version` field)
- [+] Legacy-import tests against the 4 checked-in fixtures
- [+] `cargo xtask gate` green locally: fmt, clippy `-D warnings`, tests, `cargo deny
      check`, `sync-agents --check`
- [+] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a proven-invariant
      comment
- [+] `docs/architecture/overview.md` updated
- [+] `ROADMAP.md`: `**Status.**` line under S1 + §1 table status updated; "25 полей"
      corrected to "26" in both places

## Completion

- [+] CI green on all three OSes: run #38
      (https://github.com/dimbo1324/codepack/actions/runs/29981484227),
      `gate (ubuntu-latest)` / `gate (macos-latest)` / `gate (windows-latest)` — все
      `success` на коммите `67f2513`
- [+] Commits: checklist first, then implementation, separated logically
- [+] Fast-forward merge into `main` and push to `origin` (explicit owner sign-off)
- [+] Final report to owner (Russian, per language policy)

## Mid-implementation deviations (recorded honestly)

- `cargo deny check` failed on a real license issue: `directories` (planned for
  `paths.rs`) pulls in `option-ext` (MPL-2.0, copyleft). Asked the owner rather than
  silently allow-listing a copyleft license or silently dropping the dependency —
  owner chose to implement `AppPaths` by hand over environment variables instead.
  `directories` was never added to the workspace; `docs/decisions/open-questions.md`
  records the decision.
- `cargo-deny` is not installed by default on a dev machine (not a `rust-toolchain.toml`
  component) — installed locally to verify the gate for real (`cargo install
  cargo-deny --locked`), documented the requirement in
  `.ai/project/11-commands.md`, and CI installs it via `taiki-e/install-action`.
- `[licenses.private] ignore = true` added to `deny.toml`: our own workspace crates
  have no `license` field yet (separate, undecided question) and would otherwise fail
  the license check as "unlicensed" — this is the correct cargo-deny mechanism for
  unpublished internal crates, not a loosening of the copyleft policy.
- `codepack-quality-reviewer` caught a real parity bug before merge: `normalized_ui_zoom`
  fell back to the default for all non-finite input, but legacy's `min`/`max` against
  NaN is a comparison quirk that actually clamps NaN/+inf to 1.5 and -inf to 0.7 rather
  than falling back at all. Decided not to replicate the quirk (it is an accident, not
  a behavior, and unreachable through JSON since `serde_json` rejects NaN/Infinity
  tokens) — documented the deviation explicitly in the doc comment instead of silently
  diverging. Also fixed: an inflated "56 unit + 6 integration" test-count claim (actual
  is 50 unit + 6 integration = 56 total) in ROADMAP.md/overview.md, a stale
  `directories`-crate mention left in BLUEPRINT.md §D.4 and Приложение 3 after the
  mid-task pivot, an `io.rs` doc comment overclaiming legacy parity on partial-failure
  behavior, and the two stale pre-pivot `directories` references in this file's own
  Implementation section above.

---

## Next task

Stage **S2 — Scanner: tree walking, ignore rules, stack detection (`codepack-scanner`)**
(`ROADMAP.md` §2). Start with the orientation ritual from
`.ai/project/13-progress-tracking.md`.
