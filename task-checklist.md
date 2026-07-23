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
      `16_key_files_report`/`22_project_health_report` — this is new capability
      riding along with parity content (legacy shows bytes only in these reports),
      and it means ROADMAP §1's "S7 depends on S2, S3, S4" table entry is now
      incomplete (should read "..., S6") purely as a consequence of the owner's
      stage-merge decision, to be noted honestly in S7's Status text, not silently
      absorbed
- [+] S6 confirmed: legacy's fallback token formula is `max(1, round(B/3.5))` —
      **`round`, not `ceil`** as BLUEPRINT §E.1's prose simplifies it to; the archive
      wins per `.ai/project/14-legacy-reference.md`. No "Fit to budget" or per-file
      token estimation exists anywhere in legacy — both are genuinely new (🎯).
      `tiktoken-rs` deliberately deferred (no acceptance criterion needs exact BPE).
      `codepack-tokens` designed as a pure, config-agnostic crate (no `Config`
      bridge struct — no token/budget field exists in `Config` and none is needed).
- [+] S7 scope correction confirmed against the legacy archive (ROADMAP's "Состав"
      line is not literally accurate): `28_export_plan` is already `codepack-scanner`
      (S2)'s job, done; `29_export_comparison_report` is already `codepack-diff`
      (S4)'s job, done; `27_archive_plan.md` cannot exist yet (S8 comes after S7) —
      legacy's own dashboard already tolerates its absence at this pipeline point,
      not a gap S7 introduces. S7 actually owns 26 numbered reports (01–26, excluding
      27/28/29) + `PROJECT_PROFILE.json` + `REPORT_PLUGINS.json` + `AI_CONTEXT/` +
      `AI_PROMPTS/` + `REPORT_DASHBOARD.html` + `manifest.json`/`INDEX.md` writers.
- [+] S7's `06_security_scan.*` will be a thin adapter over `codepack_security::scan`
      (S3) — legacy's own independent, weaker re-implementation of a scanner inside
      `reports/insights/security.py` is confirmed NOT to be ported (I3: no re-scanning
      raw content for secrets outside the one real detector).
- [+] S7's `05_git_deep`/`21_git_timeline_report` will use `git2` directly (workspace
      dependency already pinned by S4, vendored-libgit2, no https/ssh/cred) rather
      than depending on `codepack-diff` as a crate (that crate's surface is
      diff/selection-shaped, not general repo introspection) or shelling out to `git`
      (forbidden by domain rules)
- [+] `AI_PROMPTS/`'s other prompt files (beyond `CUSTOM_PROMPT.md`) are confirmed
      legacy-empty (the `PROMPTS` dict has no other entries in production) — BLUEPRINT's
      description of ready-made review/refactor/security/bug-hunt prompt files is
      aspirational, not a literal legacy feature; left as legacy left it, not invented
- [+] Realistic sizing accepted: S7 is sequenced by data-dependency group (G → A → B →
      D → C → E → F → G-finish), each with its own build/test checkpoint, rather than
      attempted as one unstructured pass — this is how the full stage stays achievable
      in one task, not a scope cut

## Implementation — S6 (`codepack-tokens`)

- [ ] `bytes.rs` — `format_bytes` ported 1:1 (binary units B/KB/MB/GB/TB, no PB tier,
      `.2f` for non-B units) — byte-parity regression test against legacy fixture
      values is the acceptance gate (I4)
- [ ] `tokens.rs` — `estimate_tokens_fallback` (`max(1, round(B/3.5))`, matching
      legacy's actual `round`, documented inline as a correction to BLUEPRINT's
      `ceil` prose) and `estimate_tokens_refined` (calibrated `b`/`k` coefficients,
      ASCII vs Cyrillic UTF-8 presets) — both public, fallback never replaced (I4)
- [ ] `model_limits.rs` — `ModelContextLimits` default table (4 legacy entries:
      Claude 200K, GPT-4o 128K, GPT-4 Turbo 128K, Gemini 1.5 Pro 1M), serde
      round-trip shape; no file-loading/override mechanism this stage (documented
      extension point for S9/S11)
- [ ] `budget.rs` — `BudgetCandidate { id, importance, tokens }`,
      `fit_to_budget(candidates, budget_tokens) -> BudgetSelection` with an
      inspectable `ExclusionReason`, deterministic tie-breaking rule chosen and
      tested — takes importance as a caller-supplied value, never computes it
      (S7's `16_key_files_report`/`22_project_health_report` own that)
- [ ] `lib.rs` — crate-scope doc: no importance computation, no pipeline wiring, no
      network, no override-file loading, under 100 lines

## Verification — S6

- [ ] Byte-parity test (unit-boundary values) matches legacy exactly
- [ ] Token fallback test matches legacy `round`-based values
- [ ] Refined-vs-fallback test proves a meaningfully lower estimate for Cyrillic
      UTF-8 content than the crude fallback
- [ ] Budget selector determinism test (repeat-run equality) + explainability test
- [ ] `ModelContextLimits` JSON round-trip test
- [ ] `cargo tree -p codepack-tokens` audited: no network-capable crate, no
      unjustified new dependency
- [ ] `cargo xtask gate --quick` green before moving to S7 implementation

## Implementation — S7 (`codepack-reports`), by group

- [ ] Group G (glue, built first): shared `Inventory`, `ReportContext`, plugin
      table + runner (profile gating, `catch_unwind` + `Result` handling,
      `ERROR_<name>.txt` fault isolation), `REPORT_PLUGINS.json`,
      `PROJECT_PROFILE.json` builder
- [ ] Group A (pure plan/byte reports): `01_summary`, `02_file_statistics`,
      `07_todo_fixme`, `08_code_metrics`, `25_large_files_report`
- [ ] Group B (security wrapper): `06_security_scan.{txt,json,sarif}` as a thin
      adapter over `codepack_security::scan::ScanResult`
- [ ] Group D (manifest parsers): `go.mod`/`package.json`/`requirements*.txt`/
      compose-YAML-subset parsers; `03_dependencies`, `04_scripts`, `09_config`,
      `10_docker`, `26_dependency_intelligence`
- [ ] Group C (git2 reports): `05_git_deep`, `21_git_timeline_report` — read-only
      `git2` queries, output redacted via `codepack_security::redact::redact_secrets`
- [ ] Group E (heuristic/derived reports): shared dependency-graph primitive;
      `11_routes_and_pages`, `14_dependency_graph`, `15_architecture_report`,
      `16_key_files_report`, `17_code_quality_report`, `18_api_surface_report`,
      `19_frontend_report`, `20_backend_report`, `22_project_health_report`,
      `23_refactoring_opportunities`, `24_architecture_map`
- [ ] Group F (AI bundle): `12_ai_context_pack`, `13_runbook`, `AI_CONTEXT/` (11
      files), `AI_PROMPTS/CUSTOM_PROMPT.md`
- [ ] Group G finish: `REPORT_DASHBOARD.html` (tolerates missing
      `27_archive_plan.md`/`28_export_plan.md`), `manifest.json`/`INDEX.md` writer
      functions, RU/EN string-table localization piloted on one report

## Verification — S7

- [ ] Golden-structure tests per group against stack fixtures (right files, right
      top-level shape — not byte-identical, since content is project-dependent)
- [ ] Profile-gating test for all 5 profiles against the reconstructed table
- [ ] Fault-tolerance test: forced single-report failure → exactly one
      `ERROR_<name>.txt`, every other report/artifact still completes
- [ ] I3 audit: planted secret in a fixture never appears raw in any report output;
      `<REDACTED>` placeholder appears where expected
- [ ] Confirmed no `std::process::Command` invoking `git` anywhere in this crate
- [ ] `PROJECT_PROFILE.json`/`manifest.json` field sets matched against the
      extracted legacy archive, not from memory
- [ ] `cargo xtask gate` green (fmt, clippy `-D warnings`, full test suite, `cargo
      deny check`, `sync-agents --check`)
- [ ] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment

## Completion (both stages)

- [ ] `docs/architecture/overview.md` updated (both crates move from placeholder)
- [ ] `ROADMAP.md` — separate `**Status.**` lines for S6 and S7 + §1 table, each
      honestly listing its own deviations (S6: round-vs-ceil, no legacy budget
      precedent; S7: the 27/28/29 scope correction, the S6 dependency-table drift,
      the AI_PROMPTS legacy-emptiness gap, the git2-not-codepack-diff choice)
- [ ] `docs/decisions/open-questions.md` updated with any new open questions
      surfaced (e.g. AI_PROMPTS content, REPORT_PLUGINS.json description
      population, whether to parse Cargo.toml, deferred tiktoken-rs)
- [ ] Independent review pass (`codepack-quality-reviewer`) before merge
- [ ] CI green on all three OSes; merge only after explicit owner sign-off
- [ ] Commits: checklist first, then implementation (logically separated by
      stage/group where practical), separated from documentation commits
- [ ] Final report to owner (Russian, per language policy)

---

## Next task

Stage **S8 — Архивация (`codepack-archive`)** (`ROADMAP.md` §2). Start with the
orientation ritual from `.ai/project/13-progress-tracking.md`.
