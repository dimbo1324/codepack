# Task Checklist

**Task:** Stages **S6 — Байты, токены, бюджет (`codepack-tokens`)** AND **S7 — Отчёты
и аналитика (`codepack-reports`)** (`ROADMAP.md` §2), combined into **one branch, one
task** per explicit owner instruction in this conversation — a deliberate one-off
deviation from the project's normal "one stage = one task/branch" rule
(`.ai/universal/07-multi-assistant.md`: an explicit owner instruction in the current
conversation overrides both project and universal modules). Both stages still get
their own `**Status.**` line in `ROADMAP.md` and are tracked as logically separate
within this one checklist.
**Date:** 2026-07-23
**Branch:** feat/s6-s7-tokens-and-reports

## Preparation (both stages)

- [+] Orientation ritual confirmed (git status/log, ROADMAP §1, overview.md,
      task-checklist.md, open-questions.md) — S6 is the first stage without a
      `**Status.**` line; no blocking open item
- [+] Delegated planning to `codepack-stage-planner` for S6 and S7 in parallel
      (legacy archive extracted to scratchpad and read directly for both)
- [+] Reconciled the two plans: S7 has a real, justified Cargo dependency on S6's
      public API (`format_bytes`, token estimates) for `01_summary`/
      `16_key_files_report`/`22_project_health_report` — new capability riding along
      with parity content; ROADMAP §1's "S7 depends on S2, S3, S4" table entry is now
      incomplete (should read "..., S6") purely as a consequence of the owner's
      stage-merge decision — noted honestly in S7's Status text
- [+] S6 confirmed: legacy's fallback token formula is `max(1, round(B/3.5))` —
      **`round`, not `ceil`** as BLUEPRINT §E.1's prose simplifies it to. No "Fit to
      budget" or per-file token estimation exists anywhere in legacy — both genuinely
      new (🎯). `tiktoken-rs` deliberately deferred. `codepack-tokens` is a pure,
      config-agnostic, dependency-free crate (no `Config` bridge, no `codepack-core`
      dependency at all).
- [+] S7 scope correction confirmed against the legacy archive: `28_export_plan`
      (S2) and `29_export_comparison_report` (S4) already done; `27_archive_plan.md`
      cannot exist yet (S8 comes after S7). S7 actually owns 26 numbered reports
      (01–26) + `PROJECT_PROFILE.json` + `REPORT_PLUGINS.json` + `AI_CONTEXT/` +
      `AI_PROMPTS/` + `REPORT_DASHBOARD.html` + `manifest.json`/`INDEX.md` writers.
- [+] S7's `06_security_scan.*` is a thin adapter over `codepack_security::scan` (S3).
      `05_git_deep`/`21_git_timeline_report` use `git2` directly (workspace-pinned,
      vendored-libgit2, no https/ssh/cred) — no subprocess, statically verified.
- [+] `AI_PROMPTS/`'s other prompt files (beyond `CUSTOM_PROMPT.md`) confirmed
      legacy-empty — left as legacy left it, not invented.
- [+] Realistic sizing accepted: S7 sequenced by data-dependency group (G → A → B →
      D → C → E → F → G-finish), each with its own build/test checkpoint.

## Implementation — S6 (`codepack-tokens`)

- [+] `bytes.rs` — `format_bytes` ported 1:1, byte-parity regression test passes (I4)
- [+] `tokens.rs` — `estimate_tokens_fallback` (`max(1, round(B/3.5))`) and
      `estimate_tokens_refined` (ASCII/Cyrillic-UTF8 coefficient presets) — both
      public, fallback never replaced (I4)
- [+] `model_limits.rs` — `ModelContextLimits` default table (4 legacy entries),
      serde round-trip shape; no file-loading/override mechanism this stage
- [+] `budget.rs` — `BudgetCandidate`/`fit_to_budget`/`ExclusionReason`, deterministic
      tie-breaking, importance always caller-supplied
- [+] `lib.rs` — crate-scope doc, 32 lines, no dependency on any `codepack-*` crate

## Verification — S6

- [+] Byte-parity test matches legacy exactly at every boundary
- [+] Token fallback test discriminates `round` vs `ceil` (B=4 case)
- [+] Refined-vs-fallback test: 286 vs 211 tokens for a 1000-byte Cyrillic sample
- [+] Budget selector determinism + explainability + tie-break tests
- [+] `ModelContextLimits` JSON round-trip test
- [+] `cargo tree -p codepack-tokens`: only `serde`(+derive)/`serde_json`(dev) — no
      network-capable crate, no dependency on `codepack-core`
- [+] `cargo xtask gate` green — 18 tests in `codepack-tokens`

## Implementation — S7 (`codepack-reports`), by group

- [+] Group G (glue): `Inventory`, `ReportContext`, `ReportJob`/`run_reports` (profile
      gating, cancellation between jobs, `catch_unwind` + `Result` fault isolation,
      `ERROR_<name>.txt` — including legacy's own double-extension naming quirk,
      verified against `orchestrator.py`), `REPORT_PLUGINS.json`,
      `PROJECT_PROFILE.json` builder
- [+] Group A: `01_summary`, `02_file_statistics`, `07_todo_fixme`, `08_code_metrics`,
      `25_large_files_report`
- [+] Group B: `06_security_scan.{txt,json,sarif}` — thin adapter over
      `codepack_security::scan::ScanResult`; writes a `.txt`-only placeholder when
      scan data isn't supplied, never an empty-but-valid `.json`/`.sarif`
- [+] Group D: `go.mod`/`package.json`/`requirements*.txt`/compose-YAML-subset
      parsers; `03_dependencies`, `04_scripts`, `09_config`, `10_docker`,
      `26_dependency_intelligence`. `Cargo.toml` deliberately never parsed
      (confirmed legacy parity choice, not an oversight).
- [+] Group C: `05_git_deep`, `21_git_timeline_report` via read-only `git2` queries;
      output redacted; both degrade gracefully outside a git repository
- [+] Group E: shared import-graph primitive (`graph.rs`); `11_routes_and_pages`,
      `14_dependency_graph`, `15_architecture_report`, `16_key_files_report`,
      `17_code_quality_report`, `18_api_surface_report`, `19_frontend_report`,
      `20_backend_report`, `22_project_health_report`,
      `23_refactoring_opportunities`, `24_architecture_map` — legacy scoring
      weights/formulas ported exactly (independently re-verified by review), a few
      documented approximations where legacy's own approach (AST parsing, absolute
      path checks) has no clean Rust equivalent without an unjustified dependency
- [+] Group F: `12_ai_context_pack`, `13_runbook`, `AI_CONTEXT/` (11 files),
      `AI_PROMPTS/CUSTOM_PROMPT.md` (driven by `Config.prompt_goals`) — confirmed
      `AI_PROMPTS/` produces only `CUSTOM_PROMPT.md`, matching legacy's real (not
      aspirational) feature
- [+] Group G finish: `REPORT_DASHBOARD.html` (tolerates missing
      `27_archive_plan.md`/`28_export_plan.md`, matching legacy's own fallback text),
      `manifest.json`/`INDEX.md` pure writer functions (awaiting S9's real pipeline
      data, mirroring `codepack-diff::write_diff_report`'s precedent), RU/EN
      string-table localization piloted on `01_summary.txt` only (deliberately not
      wired to the UI-language config field — retrofitting the other ~25 reports is
      explicitly deferred, not silently dropped)

## Verification — S7

- [+] Golden-structure tests per group against stack fixtures; a full-catalog
      end-to-end test for the `full` profile plus 2 additional profiles cross-checked
      against the reconstructed gating table (independently re-verified against
      `orchestrator.py` by review — exact match, no discrepancies)
- [+] Fault-tolerance tests per group: forced `Err` **and** forced `panic!` jobs both
      produce `ERROR_<name>.txt`; every other job in the same run still completes
- [+] I3 audit: crate-wide sweep test plants secrets in a script, a docker-compose
      config, a git commit message, and a TODO comment, and greps the entire output
      tree — **a real, undisclosed gap was found and fixed during independent
      review** (see Completion section below)
- [+] Confirmed zero `std::process::Command` invoking `git` anywhere in this crate
      (a static source-scan test, `tests/group_c_e_runner.rs`)
- [+] `PROJECT_PROFILE.json`/`manifest.json` field sets matched against the extracted
      legacy archive
- [+] `cargo xtask gate` green — 139 tests in `codepack-reports` (workspace-wide gate
      also green with both new crates present)
- [+] No `unsafe`; every `unwrap()`/`expect()` outside tests carries a
      proven-invariant comment

## Completion (both stages)

- [+] `docs/architecture/overview.md` updated (both crates move from placeholder)
- [+] `ROADMAP.md` — separate `**Status.**` lines for S6 and S7 + §1 table
- [+] `docs/decisions/open-questions.md` updated (Q11: `MODEL_CONTEXT_LIMITS`
      override-file loading deferred to S9/S11; Q12: whether/when to retrofit
      RU/EN localization across the remaining ~25 reports; Q13: whether/when to
      close the per-report-loop cancellation gap once S9 wires a live caller)
- [+] Independent review pass (`codepack-quality-reviewer`, run as two parallel
      passes — S6 and S7 separately given S7's size) — S6: clean, two minor
      documentation-drift notes (folded into the Status text and open questions
      below), no code defects. S7: found and fixed one real, undisclosed I3
      narrowing — `redact_line` called the weaker `codepack_security::redact_secrets`
      instead of the stronger `patterns::keyword::redacted_line` legacy's own
      `docker_report.py` actually used, silently missing `DATABASE_URL`/`JWT_SECRET`/
      `ACCESS_KEY`/`CLIENT_SECRET` — empirically confirmed via a leaked
      `DATABASE_URL=...` line in `10_docker.txt`, fixed, and covered by a new
      regression test. Also found and **honestly documented (not fixed)**: report
      jobs check cancellation only between whole jobs, not inside their own per-file
      loops (`.ai/project/12-domain-rules.md`) — judged lower-risk to disclose as
      acknowledged tech debt (crate has no live caller yet; S9 is the real
      integration point) than to retrofit ~19 modules' internal loops during the
      final pass before merge. Everything else the review checked (profile-gating
      table, `16_key_files`/`22_project_health` scoring formulas, `AI_PROMPTS`
      legacy-emptiness, git2/no-subprocess, fault tolerance, architecture boundaries,
      scope) was independently re-derived and confirmed correct.
- [+] CI green on all three OSes — confirmed live (run #48, commit `cab79df`,
      `gate (ubuntu-latest)`/`gate (macos-latest)`/`gate (windows-latest)` all
      `success`)
- [+] Commits: checklist first, then S6, then S7 (four passes: G+A, B+D, C+E,
      F+G-finish), then the review-driven fix commit, separated logically
- [+] Fast-forward merge into `main` — done, with explicit owner sign-off; rebased
      cleanly onto `main` first (main had advanced by one commit, the S5 CI
      confirmation), then merged and pushed as `cab79df`
- [+] Final report to owner (Russian, per language policy)

---

## Next task

Stage **S8 — Архивация (`codepack-archive`)** (`ROADMAP.md` §2). Start with the
orientation ritual from `.ai/project/13-progress-tracking.md`.
