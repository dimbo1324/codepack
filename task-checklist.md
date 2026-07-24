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

- [+] `error.rs` — `ArchiveError` (thiserror), `Result` alias
- [+] `options.rs` — `ArchiveOptions` + `From<&codepack_core::config::Config>`
      (`include_project_in_zip`, `effective_zip_part_bytes()`)
- [+] `entry.rs` — `ArchiveEntry`, `classify_archive_group` (exact 14-group port,
      exact first-match-wins priority order — `reports/`-prefix and
      test-component checks run *before* extension-based checks), `collect_entries`
      (cancellation checked per file inside the walk loop)
- [+] `plan.rs` — `ArchivePartPlan`/`ArchivePlan`, `plan_archive` (First-Fit-by-group
      at target 500MB/hard limit 512MB/8MB reserve, large-file-gets-own-part,
      deterministic `(group, arcname.casefold())` sort), `predicted_result_for_plan`
- [+] `report.rs` — `write_archive_plan_report` (`27_archive_plan.md`/`.json`,
      self-contained, no `codepack-reports` dependency)
- [+] `build.rs` — `build_final_archives`: 3-pass re-plan + `on_plan_ready` hook at
      the two exact legacy call sites (before real write; on the
      exceeded-after-write retry, **re-walking after both hook calls**, fixed
      during review — see Completion notes), single-ZIP write at `ZIP_DEFLATED`
      level 6, post-write hard-limit check + delete-and-rebuild-as-split fallback,
      split-part write loop with per-entry cancellation, `oversized_files`
      reporting (no recursive re-split, matches legacy)
- [+] `restore.rs` — `safe_member_target` (lexical component-based path validation —
      no `canonicalize()`, which requires the path to already exist),
      `extract_zip_safely` (dual check via `ZipFile::enclosed_name()` +
      the lexical check, fail-closed on the first bad entry, matching legacy's
      abort-not-skip behavior), `restore_archive_set`,
      `ARCHIVE_SET_MANIFEST.json`/`RESTORE_INSTRUCTIONS.md` writers — a Rust
      library function replaces legacy's bundled `restore_archives.py` script
- [+] `lib.rs` — crate-scope doc stating the S8 scope boundary, mirrors
      `codepack-diff`'s style, under 100 lines

## Verification — S8

- [+] `classify_archive_group` unit tests: all 14 groups + priority-order edge cases
      (reports/-prefix beats extension, singular/plural "test" at any depth,
      dockerfile-prefix match, `00_metadata` casefold matching)
- [+] `plan_archive` unit tests: same-group bundling, group-switch flush, large-file
      isolation, deterministic part indices
- [+] Single-ZIP round-trip integration test (byte-for-byte, level-6 deflate
      verified via the `zip` crate's own read-back metadata)
- [+] Split-set integration test (tiny `part_limit_bytes`, multiple parts, manifest/
      restore-instructions written and parse correctly)
- [+] Restore round-trip integration test (split set → fresh directory → contents
      match exactly)
- [+] Single-exceeded-after-write retry test (near-zero-byte file + tiny limit,
      deterministic recipe — the ZIP container overhead alone exceeds the limit;
      genuinely exercises the retry, confirmed by review via `hook_calls == 2`)
- [+] Security test: malicious entry paths (`../../x`, absolute, embedded-`..`)
      rejected before any write outside the destination directory; a legitimate
      entry before the malicious one in the same archive is still safely written
      (honest partial-safe-prefix behavior, ported not silently changed)
- [+] Cancellation test: pre-cancelled token yields a partial, honestly-incomplete
      result (I6-adjacent — never a false "complete" result). Mid-loop
      (post-start) cancellation is checked inside `collect_entries`/`write_zip`'s
      loops (confirmed by code reading) but not exercised by a dedicated
      timing-based test — judged not worth the flakiness risk of a thread-timed
      test for an already-code-confirmed property; disclosed, not silently skipped.
- [+] `cargo tree -p codepack-archive`: only `codepack-core` + `zip` (+ its own
      transitive deps, narrowed to `deflate-flate2-zlib-rs` — no unused `zopfli`
      backend) — no network-capable crate, no new `deny.toml` exception
- [+] `cargo xtask gate` green (fmt, clippy `-D warnings`, tests, `cargo deny check`,
      `sync-agents --check`)
- [+] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment

## Completion — S8

- [ ] `docs/architecture/overview.md` updated (`codepack-archive` moves from
      placeholder) — deferred to the final combined completion pass (with S9)
- [ ] `ROADMAP.md` — S8 `**Status.**` line + §1 table — deferred to the final
      combined completion pass
- [ ] `docs/decisions/open-questions.md` updated if the restore-script substitution
      or First-Fit-Decreasing deferral needs a recorded open question — deferred
- [+] Independent review pass (`codepack-quality-reviewer`) — found and fixed: (1)
      the commit message overclaimed that `ZipFile::enclosed_name()` alone is
      insufficient against path traversal — reviewer's own falsifiability test
      (removing `safe_member_target`) showed the opposite for the three tested
      payloads; both checks are legitimate defense-in-depth, kept, but the
      overclaiming language corrected; (2) a real parity gap — the single→split
      retry path reused stale pre-hook entries instead of re-walking after the
      second `on_plan_ready` call, silently dropping content that call itself
      wrote — fixed, now re-walks after both hook calls, matching legacy; (3) the
      `zip` feature selection pulled in an unused `zopfli` backend — narrowed;
      (4) a misattributed Q7 citation in a doc comment — corrected.

---

## Preparation — S9

- [+] Orientation ritual re-confirmed at planning start (git status/log, ROADMAP,
      overview.md, task-checklist.md, open-questions.md) — S8 real code (incl.
      review fixes), S9 next
- [+] **Major scope finding**: no existing crate implements pipeline steps 2 (copy),
      3 (structure report), 4 (Git report), or 5 (text dump), or an `ExportPaths`
      constructor — confirmed via grep across every crate and via explicit
      "arrives with S9"/"S9's job to combine" doc comments already left in S1-S3.
      S9 is roughly half new pipeline-step logic, half orchestration — not a thin
      glue layer, comparable in size to S7. Recorded honestly rather than
      discovered mid-implementation.
- [+] Design decision: copy step (2) derives its file list from
      `ExportPlan.included_files` (already symlink-safe, already ignore-rule
      filtered by S2) + safety/diff filtering — not a second independent tree walk
      re-implementing ignore-directory logic a third time
- [+] Design decision: step 6's `Inventory`/`ReportContext` is built from a second,
      cheap `build_export_plan` pass over the copy (`project_dir`) — avoids adding
      new API surface to the already-shipped, reviewed `codepack-reports` crate
- [+] Design decision: `build_export_paths` takes an explicit `output_root: &Path`
      parameter rather than hardcoding legacy's Windows-Desktop assumption —
      default-root resolution deferred to S10/S11
- [+] Design decision: `import_legacy_history` triggering is explicitly OUT of S9's
      scope — a first-run CLI/UI concern (S10/S11), not a per-export-run one
- [+] `encoding_rs` dependency justified (6-encoding text-dump fallback chain:
      utf-8/utf-8-sig/cp1251/cp866/utf-16/latin-1) — MIT/Apache-2.0, no copyleft
      concern
- [+] Q13 (report-loop cancellation gap, from S7) — decision: address for the
      heaviest report loops if time allows during S9's own cancellation-suite
      work; otherwise re-record honestly rather than silently claim closed
- [+] Success/failure gate confirmed from legacy: `!cancelled && copy_stats.errors
      == 0` — feeds `codepack_storage::record_export_run`'s `Option<Snapshot>`
      gate (I6) and a future S10 CLI exit code
- [+] `on_plan_ready` closure design confirmed: constructed in S9, calls
      `codepack_reports::metadata::write_manifest`/dashboard refresh, safe to be
      invoked 1-2 times by `build_final_archives` itself plus one more direct call
      by S9 after archiving returns (matches legacy's final
      `refresh_bundle_metadata` call)
- [+] Token estimate for history: `estimate_tokens_fallback` (legacy's flat-ratio
      formula), never `estimate_tokens_refined` — silently swapping would be an
      undisclosed behavior change, not an improvement (I4)

## Implementation — S9 (`codepack-engine`), sequenced by group

- [+] Group P (paths + plan): `build_export_paths` (explicit `output_root` param,
      UTC compact-stamp collision loop, `sanitize_name`), step 1 wiring
      (`run_export_plan`: `ExportIgnoreRules` + file-override application,
      `build_export_plan`, `resolve_diff_selection` with `previous_snapshot` as a real
      parameter, `combined_selected_paths` minus the permanently-dead
      `incremental_selection` branch, `28_export_plan`/`29_export_comparison_report`
      writers) plus a new `ignored_dir_names_for` helper (`Config` has no
      `effective_ignored_dirs()` equivalent yet). **Not done**: the
      `StoredSnapshot → codepack_diff::Snapshot` converter — `codepack-storage` isn't
      wired into this crate yet (that is Group Z's job once the storage dependency is
      added); `run_export_plan` already takes `previous_snapshot:
      Option<&codepack_diff::Snapshot>` as a real parameter so Group Z only needs to
      supply a real value, not touch this function's signature.
- [+] Group C (copy): `copy_project`, safety/diff/override filtering off
      `ExportPlan.included_files`, cross-platform backslash-path reconstruction
      (`to_relative_path`: splits on `\\`, never `Path::new(rel_str)` directly).
      Documented `CopyStats` semantic shifts from the no-second-walk redesign:
      `symlinks_skipped` always `0` (already enforced at scan time),
      `dirs_skipped` sourced from `export_plan.skipped_dirs.len()`, `files_skipped`
      only reachable via the safety-skip branch (paired with
      `files_skipped_by_safety`; the diff-skip branch touches only
      `files_skipped_by_diff`, matching legacy's separate counter).
- [+] Group R (structure + Git + text dump): `structure::write_structure_report`
      (manual `os.walk`-shaped recursion, PowerShell `Get-ChildItem`-style listing,
      `ps_date` rendered in UTC not legacy's local wall clock — informational field,
      documented, same precedent as `crate::timestamp`); `git_report::write_git_report`
      (read-only `git2` equivalents of `status --short --branch`/`branch
      --show-current`/`log --oneline -5`/`show --stat --name-status HEAD`/`show --patch
      --find-renames HEAD`, via `Repository::discover` not `::open` — matches real `git`
      CLI's upward `cwd` search, same call `codepack-diff`'s own `discover_repository`
      already established; no fabricated `exit_code`/stdout/stderr framing, since none
      of that is real for a `git2` call — **redaction strengthened to
      `codepack_security::patterns::keyword::redacted_line`**, not legacy's own weaker
      `redact_secrets`, following S7's own already-corrected precedent for exactly this
      class of bug); `text_dump::write_text_dump` (6-encoding fallback chain via
      `encoding_rs`, documented as an **honest approximation, not exact parity** — see
      its own module doc comment for why `encoding_rs`'s total codecs can't reproduce
      Python's `UnicodeDecodeError`-driven fallthrough bit-for-bit; redaction
      deliberately left at the plain `redact_secrets`, matching legacy exactly, since
      the DATABASE_URL bug S7 found was specific to `docker_report.py`'s call site, not
      this one). Added a shared `relpath::to_relative_path` module (`copy.rs`'s helper,
      generalized) and extended `timestamp.rs` with `human_now_utc`/
      `human_from_system_time` plus a `pub(crate)`-visible `civil_from_days`.
- [+] Group A (analytics + manifest): `analytics::run_analytics` (re-plan-on-copy over
      `paths.project_dir`, the **only** call site for `codepack_security::scan_project`
      in the whole pipeline; `ReportContext` assembly; full seven-group job catalog
      chained in BLUEPRINT §A.7 order with `group_g_finish_jobs` last;
      `write_report_plugins_json`/`write_project_profile_json`/`run_reports`).
      `write_custom_prompt` needs no separate call site: it is already
      `codepack_reports::reports::ai_prompts::JOB`, one of `group_f_jobs()`'s members,
      and runs through `run_reports` like every other job. `manifest::write_manifest_and_index`
      (first call, `archive_result: None`) designed for Group Z's second call from the
      start — proven by this pass's own "call twice" test, not left for Group Z to
      discover. Added `ignored_dirs::extra_ignored_display` (legacy's `extra_ignored`
      variable) for `ManifestInput`/`IndexInput`/structure/Git report consumption —
      **not yet wired into a real call site** (that is Group Z's top-level orchestrator);
      carries a disclosed, temporary `#[allow(dead_code)]` until that wiring lands.
- [+] Group Z (archive + storage close-out): `build_final_archives` +
      `on_plan_ready`/dashboard-refresh closure + final post-archive refresh,
      success/failure gate, `record_export_run` wiring, staging cleanup
      (unconditional unless `keep_staging_folder`, on every code path including
      cancelled/failed), progress/log channel wiring threaded through every group.
      `AnalyticsOutcome` extended (additive) with owned `inventory`/`replan` fields so
      the step-8 hook can rebuild an equivalent `ReportContext` for
      `REPORT_DASHBOARD.html`'s mid-archiving refresh without re-planning a third
      time. `crate::storage` (new module) hand-writes both `Snapshot ↔ New*/Stored*`
      conversions — confirmed no `From` impl exists across the `codepack-diff`/
      `codepack-storage` boundary on either side; both directions round-trip-tested.
      New top-level `orchestrator::run_export` sequences all 8 steps; `ExportOutcome`
      exposes `paths`/`cancelled`/`successful`/`copy_stats`/`text_stats`/
      `archive_result`/`project_id`/`run_id`/`analytics`.
      **Real cross-stage design gap found and resolved within this pass's own scope**:
      `codepack_scanner::build_export_plan`/`codepack_diff::resolve_diff_selection`
      (S2/S4, already shipped) hard-error (`ScannerError::Cancelled`/
      `DiffError::Cancelled`) on an already-cancelled token, which would otherwise
      make `run_export` propagate `Err` instead of honoring legacy's "steps 7-8 and
      history always run" guarantee for a token cancelled before the pipeline even
      starts. Resolved by short-circuiting steps 1-2 with a synthetic, clearly-labeled
      "nothing planned/copied" `PlanOutcome` (`cancelled_before_planning_outcome`)
      whenever `cancel.is_cancelled()` is already true before step 1 is called — a
      pipeline-sequencing decision squarely within Group Z's scope, not a change to
      S2/S4's own behavior. A token that becomes cancelled *during* step 1's own
      parallel walk (a narrower timing race) is not specially handled and would still
      propagate `Err` from `run_export` — disclosed, not silently accepted as fully
      solved; judged not worth chasing given S8's own precedent for not pursuing
      every timing-dependent race with a dedicated test.
      **Deferred, disclosed**: `NewRunFile`/`NewFinding`/`NewArchivePart` population is
      real (not stubbed) whenever step 6 (`analytics`) actually ran, but silently
      empty (`&[]`) when it never ran (export cancelled before or during step 6) —
      there is no re-planned file list, scan result, or archive-part grouping to draw
      from in that case, and this is judged an acceptable, honestly-empty absence
      rather than fabricated data. `NewArchivePart.groups` is always `None` (the
      per-part group breakdown already lives in `27_archive_plan.md`/
      `ARCHIVE_SET_MANIFEST.json`; duplicating it into a third place was not worth
      this pass's time budget). `NewExportRun.redacted_count` is always `None` —
      legacy's own history JSON never tracked this field either, an honest absence
      carried forward, not a gap this pass introduces.

## Verification — S9

- [+] Shape-parity test (`tests/shape_parity.rs`): one fixture combining a `.py` file,
      a `.md` file, a `.env` file, and a small one-commit git repo, run through the
      full pipeline once — every expected top-level artifact asserted present (28/29
      plan+diff, structure/git/text-dump, all ~28 numbered reports + `PROJECT_PROFILE
      .json`/`REPORT_PLUGINS.json`/`AI_CONTEXT/`/`AI_PROMPTS/`/`REPORT_DASHBOARD.html`,
      manifest/INDEX, 27/archive plan + the final ZIP with `manifest.json`/`INDEX.md`
      inside and no leaked `.env`), shape not byte-identity, per this criterion's own
      wording. **Narrowed from the original "one fixture per stack" framing** to the
      single multi-file-type fixture this pass's own instructions specified instead —
      an explicit, requested scope change, not a shortfall.
- [+] Cancellation battery (`tests/cancellation.rs`, plus `tests/pipeline.rs`'s
      pre-existing "cancelled before step 1" test): **8 scenarios total, not a single
      file of 8** — boundary 0 (before step 1, already covered by `pipeline.rs`) plus
      7 new tests, one per boundary after steps 1 through 7. Each drives a real
      `codepack_core::CancellationToken`/progress-channel pair, cancelling
      deterministically on a real `ProgressEvent` (never a sleep). Every scenario
      asserts: no `snapshot` row advance (a pre-seeded baseline is proven row-for-row
      unchanged via `latest_snapshot`, reusing `codepack-storage`'s own
      `run.rs` assertion pattern), `run.successful = false`, staging cleanup per policy
      (both `keep_staging_folder` values exercised), and manifest/archive still exist.
      **Boundary 6 uses `StepStarted` instead of `StepFinished`** (documented in
      `tests/support/mod.rs`) to avoid a genuine last-line race at the final
      pre-step-7 `cancelled` latch. **Boundary 7 asserts a documented asymmetry**
      confirmed against legacy `exporter.py` (lines 264 vs 313): `export_run.cancelled`
      stays `false` (the field is latched before step 7 and never re-touched, matching
      legacy exactly) while the baseline is still never advanced (the `successful` gate
      freshly re-checks the token after archiving, also matching legacy). **Two real
      bugs found and fixed during this work, not merely disclosed** — see Completion
      notes below for both.
- [+] ≥50k-file synthetic-fixture performance test (`tests/perf_smoke.rs`,
      `#[ignore]`-gated, nested nowhere-near-one-flat-directory layout). Run explicitly
      in `--release` (confirmed passing twice): **50,000 files exported in ~155-157s
      wall-clock** on this pass's own sandbox (a virtualized, possibly
      antivirus-throttled Windows environment — `.ai/project/11-commands.md`'s own
      documented platform caveat). The budget was widened from an initial,
      too-optimistic 120s to 300s after measuring both a 5,000-file run (~12.9s) and
      the full 50,000-file run: a ~12x wall-clock increase for a 10x file-count
      increase is consistent with linear scaling plus a modest constant per-run
      overhead, not a quadratic blowup — this measurement is documented directly in
      the test's own module doc comment, not just here. No panic, no error, a real
      archive produced.
- [+] `last_export` mode round-trip (`tests/last_export_mode.rs`): export once (no
      prior baseline → behaves like `"all"` mode with `codepack_diff`'s own documented
      warning), edit one of three files, export again against the same `conn` — the
      second run's copied project directory contains only the edited file, and
      `29_export_comparison_report.md` names exactly that one file under "Изменённые".
      Asserted indirectly via on-disk artifacts, per this pass's own preference, rather
      than widening `ExportOutcome`'s public shape.
- [+] `cargo tree -p codepack-engine`: confirmed clean — `git2` uses
      `default-features = false` + `vendored-libgit2` (no ssh/https transport
      features); no `openssl-sys`/`libssh2-sys`/`curl`/`reqwest`/`hyper` or any other
      network-capable crate anywhere in the tree.
- [+/-] `cargo xtask gate`: `fmt`, `clippy -D warnings`, `cargo test --workspace`, and
      `sync-agents --check` all green. `cargo deny check` fails with `error: no such
      command: 'deny'` — the `cargo-deny` binary is confirmed unavailable in this
      sandbox, the same pre-existing environment gap every prior S8/S9 pass already
      hit and documented; not fixed here, per instruction.
- [+] No `unsafe` anywhere in `codepack-engine` (grepped). No bare `unwrap()`/`expect()`
      outside `#[cfg(test)]` modules (grepped file-by-file up to each file's own test
      module boundary) — confirmed clean, no remediation needed.

### Two real bugs found and fixed during this verification pass

1. **`successful` never re-checked the cancellation token after archiving.**
   `orchestrator.rs` computed `let successful = !cancelled && copy_stats.errors == 0;`
   — `cancelled` alone, the value latched *before* step 7. Legacy `exporter.py` line
   313 computes `successful = not cancelled and not self.cancel_event.is_set() and
   copy_stats.errors == 0`, deliberately re-checking the token fresh *after* steps 7-8
   complete. Without the fresh recheck, a cancellation arriving only during manifest
   writing or archiving would have been recorded as a full success and would have
   advanced the history snapshot baseline despite the user having cancelled — a real
   parity gap with data-integrity consequences (invariant I6 adjacent). Fixed to
   `!cancelled && !cancel.is_cancelled() && copy_stats.errors == 0`, restoring legacy
   parity exactly; `boundary_7_...` in `tests/cancellation.rs` exercises the fix
   directly.
2. **A cancellation race could hard-crash the whole export instead of degrading
   gracefully.** `codepack_scanner::build_export_plan`/`codepack_diff::
   resolve_diff_selection`/`codepack_security::scan_project` (S2/S3/S4, already
   shipped) hard-error on an already-cancelled token rather than cooperating. The
   orchestrator's own outer gates (`if !cancel.is_cancelled() { ... }`) only checked
   the token *before* calling into step 1's and step 6's own internal work — a
   cancellation landing in that narrow window surfaced as a hard `Err` from
   `run_export`, skipping steps 7-8 (manifest + archiving) entirely and breaking this
   pipeline's core "steps 7-8 always run" guarantee. This was previously disclosed as
   an accepted, narrow, step-1-only edge case; this pass's own cancellation battery
   hit it twice on an ordinary (non-crafted) run, proving it reachable in practice at
   step 6 too, not merely a theoretical corner. Fixed by adding
   `crate::error::is_cancellation_error` and matching on it at both call sites
   (`orchestrator.rs`, steps 1 and 6), falling back to an honestly-empty step result
   instead of propagating.

## Completion — S9

- [ ] `docs/architecture/overview.md` updated (`codepack-engine` moves from
      placeholder; first real producer/consumer of the progress/log channel)
- [ ] `ROADMAP.md` — S9 `**Status.**` line + §1 table
- [ ] `docs/decisions/open-questions.md` — Q7/Q8/Q9/Q10/Q11/Q13 resolved here or
      explicitly re-deferred with a named, honest reason (not silently dropped)
- [ ] Independent review pass (`codepack-quality-reviewer`)

---

## Completion (both stages, final)

- [ ] `docs/architecture/overview.md` and `ROADMAP.md` updated for BOTH S8 and S9
      together (S8's own completion items above were deferred to this pass)
- [ ] CI green on all three OSes; merge only after explicit owner sign-off
- [ ] Commits: checklist first, then S8, then S9 (sequenced by group), then any
      review-driven fix commits, separated logically
- [ ] Final report to owner (Russian, per language policy)

---

## Next task

Stage **S10 — CLI / headless** (`ROADMAP.md` §3). Start with the orientation ritual
from `.ai/project/13-progress-tracking.md`.
