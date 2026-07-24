# Task Checklist

**Task:** Core hardening before S10 — prove parity by *executing* legacy (golden
artifacts), wire up the three token capabilities that shipped without a caller, and
close the config-group open questions the owner decided on 2026-07-25.
This is **not** a ROADMAP stage: it is a corrective pass over the already-closed
S1–S9 core, mandated by the owner after a cross-cutting review found four
discrepancies between what the documents claim and what the code does.
**Date:** 2026-07-25
**Branch:** feat/core-hardening-golden-parity

## Preparation

- [+] Orientation ritual (git status/log, ROADMAP §1, overview.md, task-checklist.md,
      open-questions.md) — S0–S9 all carry `**Status.**` lines; S10 is next, but the
      owner directed a hardening pass first
- [+] S8+S9 closed out honestly: CI run #50 (commit `8ef3159`, all three OSes
      `success`, including the `cargo deny check` unavailable locally) recorded in
      `ROADMAP.md`
- [+] Feasibility of golden generation **proven, not assumed**: legacy `exporter.py`
      imports no PySide6; the pipeline runs headless on Python 3.14 with zero
      installed dependencies and produced all 58 artifacts in seconds
- [+] All owner decisions from 2026-07-25 recorded in
      `docs/decisions/open-questions.md` (6 new decision entries; Q3/Q4 deferred,
      Q5/Q6/Q7/Q8/Q9/Q10/Q11 resolved, Q12/Q13 explicitly left open, new Q14 for
      First-Fit Decreasing)

## Implementation — sequenced by group

### Group G — golden parity infrastructure (highest value, run first)

- [ ] `cargo xtask golden --regenerate` — runs legacy Python against the fixtures and
      writes reference artifacts into the repo. Requires Python only on a developer
      machine; **CI never runs it**
- [ ] Golden fixtures: 2-3 projects covering different stacks (at minimum Node +
      Python, ideally a mixed/monorepo case)
- [ ] Reference artifacts committed: `28_export_plan.json`, `manifest.json`,
      `PROJECT_PROFILE.json`, `06_security_scan.json`, `06_security_scan.sarif`,
      `27_archive_plan.json`, `REPORT_PLUGINS.json`, plus the full artifact-name
      listing of the produced ZIP
- [ ] Normalization layer: timestamps and absolute paths only — every other
      difference must fail the test rather than be normalized away
- [ ] Golden comparison tests wired into `cargo test --workspace`

### Group F — the I5 contract break golden already found

- [ ] `PlanSummary` regains `estimated_included_size` (formatted byte string via
      `codepack_tokens::format_bytes`) and `skipped_dirs_count` — restoring the
      legacy `28_export_plan.json` contract. No `schema_version` bump: this restores
      a contract, it does not change one

### Group Q7 — text/binary constants move to `codepack-core`

- [ ] `TEXT_EXTENSIONS`/`BINARY_EXTENSIONS`/`TEXT_FILENAMES_WITHOUT_EXTENSION`/
      `should_consider_text_file`/`looks_binary` move to `codepack-core`
- [ ] `codepack-scanner` and `codepack-security` re-export or consume from core; the
      duplicate definitions are deleted, not left behind
- [ ] Every existing constant-parity test still passes unchanged — the move must be
      provably behavior-preserving

### Group T — token capabilities get a real caller (+ Q11)

- [ ] `fit_to_budget` wired into the pipeline: budget field in `Config`, applied
      during planning/copy; excluded files carry an explainable reason
- [ ] `estimate_tokens_refined` reachable via configuration — `estimate_tokens_fallback`
      stays the default so invariant I4 and legacy history parity are untouched
- [ ] Q11: `ModelContextLimits` loadable from a JSON file via `AppPaths`, falling back
      to the built-in 4-entry table when absent
- [ ] `ROADMAP.md` §6's "B.3 Fit to budget → S6" claim corrected — it was true of the
      library, false of the product

### Group H — history retention and import get a caller (Q10)

- [ ] `keep_last_n` becomes a `Config` field, default 50 (legacy `MAX_HISTORY_ITEMS`)
- [ ] `cleanup_old_runs` called by the engine after a successful run
- [ ] `import_legacy_history` triggering decided and documented (first-run concern —
      confirm whether it lands here or genuinely belongs to S10)

### Group P — export profiles (Q8)

- [ ] `~/.project_exporter_profiles.json` read and migrated into the new format —
      a parity gap, not a new feature

### Group D — real `base..target` diff (Q9)

- [ ] `diff_target_ref` becomes a live field; `git_ref` mode compares `base..target`
      rather than always `base..HEAD`. Deviation from legacy is deliberate and must be
      documented as a 🎯 improvement, not silent drift

### Group S — security gaps

- [ ] AWS Secret Access Key signature (BLUEPRINT §B.1's second AWS signal, never
      implemented in S3)
- [ ] Corpus test re-measured; precision must not drop (invariant I9 — lowering a
      threshold to pass is forbidden)
- [ ] `redacted_count` genuinely counted and recorded in history instead of `None`

## Verification

- [ ] `cargo xtask gate` green (fmt, clippy `-D warnings`, tests, `sync-agents --check`;
      `cargo deny check` unavailable locally — CI covers it)
- [ ] Golden tests pass against the committed reference artifacts
- [ ] No test deleted, disabled, or weakened to make the gate green
- [ ] Independent review pass (`codepack-quality-reviewer`)
- [ ] CI green on all three OSes

## Completion

- [ ] `docs/architecture/overview.md` updated (constants moved to core; golden
      infrastructure; new `Config` fields)
- [ ] `ROADMAP.md` — S2/S6/S7 `**Status.**` lines amended where this task changed
      what they claim; §6 corrected for B.3
- [ ] `docs/decisions/open-questions.md` — Q7/Q8/Q9/Q10/Q11 rows updated to name the
      actual implementation that closed them
- [ ] Final report to owner (Russian, per language policy)

---

## Explicitly NOT in this task (owner decision, 2026-07-25)

- Q12 — localizing all ~25 reports RU/EN
- Q13 — cancellation inside each report's own file loop
- Q14 — First-Fit Decreasing for archive splitting
- Q6 — `.codepack.toml` (deliberately deferred to S10, where the CLI learns to read
  configs anyway)

## Next task

Stage **S10 — CLI / headless** (`ROADMAP.md` §3).
