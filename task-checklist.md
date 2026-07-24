# Task Checklist

**Task:** Stages **S8 — Архивация (`codepack-archive`)** AND **S9 — Движок-оркестратор
(`codepack-engine`)** (`ROADMAP.md` §2), combined into **one branch, one task** per
explicit owner instruction in this conversation — S9 has a hard, sequential dependency
on S8 (ROADMAP §1: S9 depends on S2–S8), unlike the largely-independent S6+S7 pairing.
S8 is planned and implemented first; S9's plan is written only once S8 is real code.
Both stages keep their own `**Status.**` line in `ROADMAP.md`.
**Date:** 2026-07-23
**Branch:** feat/s8-s9-archive-and-engine

## Preparation — S8

- [+] Orientation ritual confirmed (git status/log, ROADMAP §1, overview.md,
      task-checklist.md, open-questions.md) — S8 is the first stage without a
      `**Status.**` line; no blocking open item
- [+] Legacy archive extracted to scratchpad; `archive_service.py`, `exporter.py`
      (step 7/8 call sites), `models.py`, `constants.py` read directly — full,
      verbatim `classify_archive_group` group list confirmed (**14 groups, not
      ROADMAP's paraphrased "13"** — `60_assets_and_binary_docs`/
      `70_data_and_exports` are the exact names, not BLUEPRINT's shorthand
      `60_assets`/`70_data`; plus `80_other_project_files` and `90_other` which
      BLUEPRINT's "…" elided)
- [+] `ArchiveBuildResult` (S1, `codepack-core::types.rs`) confirmed complete as-is
      against `models.py` field-by-field, including the exact `primary_result()`
      fallback precedence — no additive fields needed for S8
- [+] `zip` crate chosen (own bundled deflate backend, no separate `flate2`
      dependency needed) — license/dependency-tree pre-check expected clean
- [+] Design confirmed: `codepack-archive` depends only on `codepack-core` (S1); the
      dashboard-refresh + manifest-refresh hook legacy fires around archiving lives
      entirely in the caller (S9) via `build_final_archives`'s `on_plan_ready`
      callback parameter — not a `codepack-archive → codepack-reports` dependency
      (documented, deliberate simplification of two legacy call sites into one hook)
- [+] First-Fit Decreasing (🎯, BLUEPRINT §E.4) deferred — no acceptance criterion
      requires it, same reasoning precedent as S6 deferring `tiktoken-rs`

## Implementation — S8 (`codepack-archive`)

- [ ] `error.rs` — `ArchiveError` (thiserror), `Result` alias
- [ ] `options.rs` — `ArchiveOptions` + `From<&codepack_core::config::Config>`
      (`include_project_in_zip`, `effective_zip_part_bytes()`)
- [ ] `entry.rs` — `ArchiveEntry`, `classify_archive_group` (exact 14-group port,
      exact first-match-wins priority order — `reports/`-prefix and
      test-component checks run *before* extension-based checks), `collect_entries`
      (cancellation checked per file inside the walk loop)
- [ ] `plan.rs` — `ArchivePartPlan`/`ArchivePlan`, `plan_archive` (First-Fit-by-group
      at target 500MB/hard limit 512MB/8MB reserve, large-file-gets-own-part,
      deterministic `(group, arcname.casefold())` sort), `predicted_result_for_plan`
- [ ] `report.rs` — `write_archive_plan_report` (`27_archive_plan.md`/`.json`,
      self-contained, no `codepack-reports` dependency)
- [ ] `build.rs` — `build_final_archives`: 3-pass re-plan + `on_plan_ready` hook at
      the two exact legacy call sites (before real write; on the
      exceeded-after-write retry), single-ZIP write at `ZIP_DEFLATED` level 6,
      post-write hard-limit check + delete-and-rebuild-as-split fallback,
      split-part write loop with per-entry cancellation, `oversized_files`
      reporting (no recursive re-split, matches legacy)
- [ ] `restore.rs` — `safe_member_target` (lexical component-based path validation —
      no `canonicalize()`, which requires the path to already exist),
      `extract_zip_safely` (primary defense via `ZipFile::enclosed_name()` +
      the lexical check, fail-closed on the first bad entry, matching legacy's
      abort-not-skip behavior), `restore_archive_set`,
      `ARCHIVE_SET_MANIFEST.json`/`RESTORE_INSTRUCTIONS.md` writers — a Rust
      library function replaces legacy's bundled `restore_archives.py` script
- [ ] `lib.rs` — crate-scope doc stating the S8 scope boundary, mirrors
      `codepack-diff`'s style, under 100 lines

## Verification — S8

- [ ] `classify_archive_group` unit tests: all 14 groups + priority-order edge cases
      (reports/-prefix beats extension, singular/plural "test" at any depth,
      dockerfile-prefix match, `00_metadata` casefold matching)
- [ ] `plan_archive` unit tests: same-group bundling, group-switch flush, large-file
      isolation, deterministic part indices
- [ ] Single-ZIP round-trip integration test (byte-for-byte, level-6 deflate
      verified via the `zip` crate's own read-back metadata)
- [ ] Split-set integration test (tiny `part_limit_bytes`, multiple parts, manifest/
      restore-instructions written and parse correctly)
- [ ] Restore round-trip integration test (split set → fresh directory → contents
      match exactly)
- [ ] Single-exceeded-after-write retry test (near-zero-byte file + tiny limit,
      deterministic recipe — the ZIP container overhead alone exceeds the limit)
- [ ] Security test: malicious entry paths (`../../x`, absolute, embedded-`..`)
      rejected before any write outside the destination directory; a legitimate
      entry before the malicious one in the same archive is still safely written
      (honest partial-safe-prefix behavior, ported not silently changed)
- [ ] Cancellation test: pre-cancelled token yields a partial, honestly-incomplete
      result (I6-adjacent — never a false "complete" result)
- [ ] `cargo tree -p codepack-archive`: only `codepack-core` + `zip` (+ its own
      transitive deps) — no network-capable crate, no new `deny.toml` exception
- [ ] `cargo xtask gate` green (fmt, clippy `-D warnings`, tests, `cargo deny check`,
      `sync-agents --check`)
- [ ] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment

## Completion — S8

- [ ] `docs/architecture/overview.md` updated (`codepack-archive` moves from
      placeholder)
- [ ] `ROADMAP.md` — S8 `**Status.**` line + §1 table
- [ ] `docs/decisions/open-questions.md` updated if the restore-script substitution
      or First-Fit-Decreasing deferral needs a recorded open question
- [ ] Independent review pass (`codepack-quality-reviewer`)

---

## S9 (`codepack-engine`) — planned once S8 is real code

This section is a placeholder until S8 lands; S9's own planning pass will replace it
with the full Preparation/Implementation/Verification/Completion breakdown, mirroring
how S7's planning happened only after S6 existed as real code (though S6/S7 were
independent — S9 has a genuine hard dependency on S8's actual public API, so this
sequencing is load-bearing here, not just tidiness).

- [ ] Plan S9 (`codepack-stage-planner`), referencing S8's actual shipped API
      (`build_final_archives`, `ArchiveOptions`, `ArchivePlan`)
- [ ] Implement the 8-step pipeline (BLUEPRINT §A.2), sequenced with gate checkpoints
      matching S7's group-by-group precedent given the integration complexity
- [ ] Verify: golden export vs. legacy fixture, cancellation at each of the 8 steps,
      performance budget on a heavy fixture (≥50k files)
- [ ] Completion: `ROADMAP.md` S9 `**Status.**` line, `docs/architecture/overview.md`,
      independent review, merge (both S8 and S9 together)

---

## Completion (both stages, final)

- [ ] CI green on all three OSes; merge only after explicit owner sign-off
- [ ] Commits: checklist first, then S8, then S9, separated logically by stage
- [ ] Final report to owner (Russian, per language policy)

---

## Next task

Stage **S10 — CLI / headless** (`ROADMAP.md` §3). Start with the orientation ritual
from `.ai/project/13-progress-tracking.md`.
