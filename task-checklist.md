# Task Checklist

**Task:** Stage **S5 — Хранилище SQLite (`codepack-storage`)** (`ROADMAP.md` §2).
**Date:** 2026-07-23
**Branch:** feat/s5-storage-sqlite-history-snapshots

Scope boundary (binding for this task): `codepack-storage` depends only on
`codepack-core` (ROADMAP §1: S5 → S1). It never depends on `codepack-scanner`/
`codepack-security`/`codepack-diff` as crates — it persists their output *shapes*
via its own locally-defined `New*` structs, which S9 (not yet built) will populate
from those crates' real types by copying fields, not via a `From` impl living here.
No wiring into the pipeline (S9's job). No computation of `Snapshot`/`Finding`/
`ExportPlan` values (S1-S4/S9's job).

## Preparation

- [+] Orientation ritual confirmed (git status/log, ROADMAP §1, overview.md,
      task-checklist.md, open-questions.md) — no blocking open item for S5
- [+] Delegated stage planning to `codepack-stage-planner`: legacy archive extracted to
      a scratchpad temp dir and read directly (`services/export_history.py`,
      `services/exporter.py`'s history-append site, `services/diff_service.py`'s
      `_last_history_snapshot`)
- [+] Confirmed via source inspection: legacy retention is `MAX_HISTORY_ITEMS = 50`,
      global across ALL projects combined, not per-project/configurable — BLUEPRINT
      overstates this the same way it overstated `incremental.py` for S4. The new
      per-project relational retention is a deliberate, logged improvement, not a
      literal port.
- [+] Confirmed via source inspection: a real legacy bug — a run that fails with copy
      errors but isn't explicitly cancelled has `cancelled: false` and `snapshot: {}`
      in its history entry; `_last_history_snapshot` returns this empty dict as if it
      were a valid (if empty) baseline, instead of falling back to "no baseline".
      Legacy JSON import must faithfully reproduce this historical shape (not "fix"
      old data), while the new write API structurally prevents the bug for new runs
      going forward (an `Option<Snapshot>` that is `None` can never be confused with
      an empty-but-present snapshot, unlike a JSON `{}`).
- [+] SQLite driver decided and justified: `rusqlite` (`bundled` feature) — no async
      runtime introduced anywhere in the workspace (matches every other crate being
      synchronous); mirrors S4's `git2`/`vendored-libgit2` precedent (statically
      compiled, no system SQLite needed); MIT/dual MIT-Apache-2.0, already in
      `deny.toml`'s allow-list, no new exception expected
- [+] Migration-runner shape decided: embedded numbered SQL strings + `schema_version`
      table, no external migration framework (`refinery`/`sqlx-migrate` would be
      overkill and the latter pulls `sqlx`)

## Implementation — schema and API (BLUEPRINT §D.2/§D.3, column-for-column)

- [ ] `Cargo.toml`: `rusqlite` (`bundled`) added to `[workspace.dependencies]` with a
      justification comment (mirrors the `git2`/`sha2` comment convention); wired into
      `crates/codepack-storage/Cargo.toml` — verify with a clean `cargo build -p
      codepack-storage` from a clean state
- [ ] `error.rs` — `StorageError` (thiserror), wraps `rusqlite::Error`/
      `std::io::Error`/`serde_json::Error` (legacy import parsing)
- [ ] `schema.rs`/`migrations/` — DDL for all 7 tables (`project`, `export_run`,
      `run_file`, `finding`, `archive_part`, `snapshot`, `snapshot_file`) +
      `schema_version`, all 4 indexes (`project(root_path)` UNIQUE,
      `export_run(project_id, started_at)`, `snapshot_file(snapshot_id, rel_path)`,
      `finding(run_id, severity)`), `ON DELETE CASCADE` on every FK; `PRAGMA
      foreign_keys=ON`/`journal_mode=WAL` set explicitly on every connection open
      (rusqlite does not persist `foreign_keys` across connections)
- [ ] `project.rs` — find-or-create by `root_path` (unique)
- [ ] `run.rs` — `record_export_run(conn, NewExportRun, files, findings,
      archive_parts, snapshot: Option<(NewSnapshot, &[NewSnapshotFile])>) ->
      Result<i64>`: one transaction; batched multi-row snapshot-file insert; snapshot
      only ever inserted, **never updated** — the concrete I6 mechanism (an
      `export_run` row is always written regardless of outcome, matching legacy's
      "always append a history entry"; a `snapshot` row is written iff the caller
      passes `Some`)
- [ ] `baseline.rs` — `latest_snapshot(conn, project_id) -> Result<Option<..>>`,
      ordered `created_at DESC LIMIT 1`
- [ ] `retention.rs` — `cleanup_old_runs(conn, project_id, keep_last_n) ->
      Result<usize>`, relies entirely on `ON DELETE CASCADE`, no manual multi-table
      deletes
- [ ] `import/` — `import_legacy_history(history_json_path, conn) ->
      Result<ImportReport>`: explicit opt-in call (not automatic on project open);
      skip-and-warn per malformed entry (never hard-fail the whole import); faithful
      (not "fixed") mapping of the poisoned-empty-snapshot case; `mtime_ns` left
      `NULL` for imported snapshot files (legacy history never persisted it);
      `run_file`/`finding` left empty for imported runs (legacy history never had
      per-file/per-finding data); one `archive_part` row per legacy `archives[]`
      entry, `compressed_bytes`/`groups = NULL`
- [ ] `codepack-core`: small additive extension to `AppPaths` — a
      `legacy_history_file()` accessor mirroring the existing
      `legacy_settings_file()`, and a settings-dir-rooted `db_file()` helper —
      reused, not reinvented
- [ ] `lib.rs` — crate doc stating the S5 scope boundary explicitly, public
      re-exports, under 100 lines

## Verification

- [ ] Migration test: clean DB reaches `schema_version = 1`; running the runner twice
      is idempotent (safe no-op); no fictitious "migrate from an older version" test
      invented — documented as N/A at this stage (S5 is schema_version 1, nothing
      predates it)
- [ ] Legacy import test against a realistic fixture history JSON (a cancelled entry,
      a successful entry, one entry shaped like the poisoned-empty-snapshot bug, one
      malformed entry) — correct `imported`/`skipped` counts and correct
      (gap-documented) row mapping
- [ ] I6 regression test: recording a run with `snapshot = None` leaves the
      previously-stored baseline snapshot completely unchanged (row-for-row, not
      just a row-count check)
- [ ] Concurrency test: two REAL on-disk-file connections (never `:memory:` for this
      class — it can't demonstrate WAL's cross-connection guarantee), WAL
      concurrent-read-during-write; plus a multi-threaded interleaved-write test
      followed by `PRAGMA foreign_key_check` returning zero violations
- [ ] Retention test: seed N+k runs for a project, `cleanup_old_runs(.., N)` leaves
      exactly N runs and cascades every child table (`run_file`/`finding`/
      `archive_part`/`snapshot`+`snapshot_file`) correctly — verified by direct
      row-count assertions per table, not just "no FK errors"
- [ ] Round-trip test: `NewFinding`/`NewSnapshotFile` fixtures shaped like real
      `codepack-security::Finding`/`codepack-diff::SnapshotFile` values map onto the
      `finding`/`snapshot_file` columns field-for-field
- [ ] `cargo tree -p codepack-storage` audited: no network-capable crate, no license
      outside `deny.toml`'s existing allow-list
- [ ] `cargo xtask gate` green locally (fmt, clippy `-D warnings`, tests, `cargo deny
      check`, `sync-agents --check`)
- [ ] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment

## Completion

- [ ] `docs/architecture/overview.md` updated
- [ ] `ROADMAP.md` `**Status.**` line under S5 + §1 table, honestly listing: the
      `MAX_HISTORY_ITEMS=50`-global-not-per-project legacy finding, the
      poisoned-empty-snapshot legacy bug and how the new schema structurally avoids
      it going forward while faithfully importing historical data as-is, the
      `mtime_ns`/`run_file`/`finding` import gaps, the `rusqlite`/`bundled` choice
- [ ] `docs/decisions/open-questions.md` updated if `keep_last_n` retention ends up as
      a `codepack-storage`-level default rather than a new `Config` field (record as
      a new open question if so)
- [ ] Independent review pass (`codepack-quality-reviewer`) before merge
- [ ] CI green on all three OSes; merge only after explicit owner sign-off
- [ ] Commits: checklist first, then implementation, separated logically
- [ ] Final report to owner (Russian, per language policy)

---

## Next task

Stage **S6 — Байты, токены, бюджет (`codepack-tokens`)** (`ROADMAP.md` §2). Start with
the orientation ritual from `.ai/project/13-progress-tracking.md`.
