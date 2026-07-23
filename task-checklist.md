# Task Checklist

**Task:** Stage **S4 — Diff и снапшоты (`codepack-diff`)** (`ROADMAP.md` §2).
**Date:** 2026-07-23
**Branch:** feat/s4-diff-snapshots-git2

Scope boundary (binding for this task): `codepack-diff` depends only on `codepack-core`
(ROADMAP §1: S4 → S1). No persistence of history/baseline (S5's job) — the previous
snapshot for `last_export` mode is a caller-supplied argument, not a lookup. No
combining of diff-selection with force-include/force-exclude or an incremental-mtime
selector (S9's `combined_selected_paths` job). No wiring into risk-preview or the real
manifest.json (S7/S9). Legacy's `incremental.py` mtime/size baseline mechanism is
confirmed dead code (zero call sites outside its own module) and is out of scope.
`diff_target_ref` is a confirmed legacy dead field (`git_ref` mode always diffs
`base..HEAD`) — parity means replicating that limitation, not "fixing" it.

## Preparation

- [+] Orientation ritual confirmed (git status/log, ROADMAP §1, overview.md,
      task-checklist.md, open-questions.md) — no blocking open item for S4
- [+] Delegated stage planning to `codepack-stage-planner`: legacy archive extracted to
      a scratchpad temp dir and read directly (`services/diff_service.py`,
      `services/incremental.py`, `services/git_diff.py`, `services/export_history.py`,
      `utils/path_utils.py::should_ignore_dir`, matching legacy test files as
      behavioral oracles)
- [+] Confirmed via grep: `incremental.py`'s selection/baseline functions have zero
      call sites outside their own module + test file — out of scope, not silently
      dropped
- [+] Confirmed via source inspection: `diff_target_ref`/`_target_ref` is unused in
      `_git_selection` — `git_ref` mode is `base..HEAD` only, verbatim
- [+] Resolved forward-dependency gap: `last_export` mode takes the previous snapshot
      as a plain function argument; no history/storage lookup lives in this crate
- [+] Resolved constants tension (Q7-shaped): ignored-directory-name set for
      `snapshot_project`'s walk is entirely caller-supplied, no hardcoded
      `IGNORED_DIR_NAMES` duplicated a third time
- [+] `git2` feature selection decided and justified: `default-features = false`,
      `features = ["vendored-libgit2"]` — no `https`/`ssh`/`cred` (no remote git ops,
      ever, per I1); `sha2` added for streamed file hashing (BLUEPRINT §C.3), not
      git2's internal object hashing

## Implementation — parity first

- [+] `Cargo.toml`: `git2` (vendored-libgit2 only) and `sha2` added to
      `[workspace.dependencies]` with justification comments; wired into
      `crates/codepack-diff/Cargo.toml` — verified with a clean `cargo build -p
      codepack-diff` from a clean state
- [+] `error.rs` — `DiffError` (thiserror), wraps `git2::Error`/`std::io::Error`
- [+] `snapshot/` — `Snapshot`/`SnapshotFile` (serde, fields matching `SNAPSHOT_FILE`
      schema: `rel_path`, `sha256`, `size`, `loc`, `mtime_ns`), streamed 1 MiB-chunk
      SHA-256, minimal ignored-dir-name (caller-supplied) + no-symlink walk with
      `CancellationToken` checked inside the loop, backslash-normalized relative
      paths matching legacy
- [+] `selection/` — `DiffSelection`/`DiffFile`/`FileStatus`; `all` (no filter);
      `last_export` as `diff_against_snapshot(current, previous: Option<&Snapshot>)`;
      `git_ref` and `uncommitted` via `git2` (rename detection, added/modified/
      deleted/renamed classification, `base` default `"HEAD"`); git-error/not-a-repo
      fallback to `all` + warning, one shared implementation (`git_common.rs`) reused
      by both modes
- [+] `report.rs` — `write_diff_report()` → Markdown matching legacy's
      `29_export_comparison_report.md` section structure and 500-item truncation
- [+] `DiffOptions` + `From<&codepack_core::config::Config>` (mirrors
      `SecurityOptions`/`ScanOptions`); `diff_target_ref` deliberately absent as a
      field, not merely unused
- [+] `lib.rs` — crate doc stating the S4 scope boundary explicitly, public re-exports,
      41 lines

## Verification

- [+] Git-mode tests use `git2`-created repositories only (`Repository::init` +
      `Index`/commits) — no `git` binary dependency, no skip-if-missing-tool weakness
      inherited from legacy
- [+] Golden-equivalent cases: add/modify/delete for `last_export`; uncommitted
      including untracked; rename with `old_path` preserved (a real `git2` quirk was
      found and fixed here — `StatusEntry::path()` returns a rename's *old* path;
      fixed by deriving new/old paths from the `DiffDelta` directly); paths with
      spaces and Cyrillic Unicode; `git_ref` diffs `base..HEAD` only
- [+] Streamed-hash test (multi-chunk-boundary correctness) and a symlink-not-followed
      test (programmatic dir-symlink escape, honest skip-and-log when the OS/
      permissions can't create one)
- [+] Cancellation test — **strengthened after independent review**: the first version
      only cancelled the token *before* calling `snapshot_project`, covering just the
      loop's first check. Added a genuine mid-walk regression
      (`snapshot_project_cancelled_mid_pass_yields_cancelled_not_a_partial_snapshot`)
      that lets candidate collection finish, then flips the token partway through the
      per-file hash+LOC pass via the caller-supplied predicate, proving the result is
      still `Err(Cancelled)`, never a `Snapshot` that looks complete.
- [+] `cargo tree -p codepack-diff` audited: only `bitflags`/`libc`/`libgit2-sys`/
      `log`/`libz-sys`/`cc`/`pkg-config`/`vcpkg` under `git2` — no `openssl-sys`,
      `libssh2-sys`, `curl`, `reqwest`, `hyper`; no `https`/`ssh`/`cred` features
      resolved. Source grepped for `Remote`/`fetch`/`push`/`clone`/`Cred` — zero hits.
- [+] `cargo xtask gate` green locally (fmt, clippy `-D warnings`, tests — 40 passing
      in `codepack-diff`, `cargo deny check`, `sync-agents --check`)
- [+] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment

## Completion

- [+] `docs/architecture/overview.md` updated (first C-library dependency in the
      workspace, noted explicitly)
- [+] `ROADMAP.md` `**Status.**` line under S4 + §1 table, honestly listing: the
      incremental.py dead-code finding, the diff_target_ref dead-field finding, the
      baseline-as-parameter resolution, the narrower I6 scope at this stage
- [+] New open question recorded in `docs/decisions/open-questions.md`: **Q9** — whether
      to later honor `diff_target_ref` as new (🎯) capability instead of replicating
      its legacy dead-field limitation
- [+] Independent review pass (`codepack-quality-reviewer`) before merge — confirmed
      I1/I2/I6/I7 compliance, the `git2` rename-path fix, and both legacy dead-code
      claims independently against the archive. One real gap found and fixed: the
      cancellation test didn't actually test mid-walk cancellation (see Verification
      above). Two findings were about this checklist/ROADMAP/overview not being
      filled in yet at review time — resolved by this very completion pass.
- [ ] CI green on all three OSes — pending merge/push, needs owner sign-off first
- [+] Commits: checklist first, then implementation, then the review-driven test fix,
      separated logically
- [ ] Fast-forward merge into `main` — pending owner sign-off
- [+] Final report to owner (Russian, per language policy)

---

## Next task

Stage **S5 — Хранилище SQLite (`codepack-storage`)** (`ROADMAP.md` §2). Start with the
orientation ritual from `.ai/project/13-progress-tracking.md`.
