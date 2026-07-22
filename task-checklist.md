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

- [+] Added `serde`, `serde_json`, `thiserror`, `directories`, `crossbeam-channel` to
      `[workspace.dependencies]`; `tempfile` as a dev-dependency; wired into
      `crates/codepack-core/Cargo.toml` via `dep.workspace = true`
- [+] `crates/codepack-core/src/error.rs` — `CoreError` (thiserror) + `Result<T>` alias
- [+] `crates/codepack-core/src/types.rs` — `ExportPaths`, `CopyStats`, `TextDumpStats`,
      `RiskPreviewItem`, `RiskPreviewReport` (+ `has_warnings`), `ArchiveBuildResult`
      (+ `primary_result`)
- [+] `crates/codepack-core/src/cancellation.rs` — `CancellationToken`
- [+] `crates/codepack-core/src/progress.rs` — `ProgressEvent`/`LogEvent` +
      crossbeam-channel aliases
- [+] `crates/codepack-core/src/paths.rs` — `AppPaths` over `directories`, injectable
      base dir for hermetic tests, legacy settings-file location resolver
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

- [ ] CI green on all three OSes (confirm after push)
- [+] Commits: checklist first, then implementation, separated logically
- [ ] Fast-forward merge into `main` (after explicit owner sign-off, per workflow)
- [ ] Final report to owner (Russian, per language policy)

---

## Next task

Stage **S2 — Scanner: tree walking, ignore rules, stack detection (`codepack-scanner`)**
(`ROADMAP.md` §2). Start with the orientation ritual from
`.ai/project/13-progress-tracking.md`.
