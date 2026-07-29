# Task Checklist

**Task:** Three "smarter AI handoff" features proposed and approved 2026-07-29, all in
one branch (owner instruction):

- **B1** — `--budget` by model name: `--budget "Claude (200K)"` resolves through
  `ModelContextLimits`. This also finally gives that table a real consumer: built in S6,
  given a file-override mechanism during the hardening pass, and to this day mentioned
  only by `doctor`.
- **B2** — `codepack explain <file>`: why a file did or did not make it into the export.
  `PlannedFile::reason` already holds the answer; there is no way to ask for it.
- **B3** — a sixth AI preset for reviewing a pull request: composed entirely of pieces
  that already exist (`diff_export_mode: "uncommitted"` + the `ai_review` report
  profile + the git patch). No new mechanics.

**Date:** 2026-07-29
**Branch:** feat/budget-by-model-explain-pr-preset

Owner approved, at the end and only once everything is green: push to `origin`, merge
into `main`, delete every branch except `main`.

## Preparation

- [+] Orientation: git status/log, `docs/architecture/overview.md`, previous
      `task-checklist.md` (closed cleanly), `docs/decisions/open-questions.md`
- [+] Read the real code for each of the three before designing, not after
- [+] **B1 correction found while reading**: the proposal assumed keys like
      `claude-opus-5`, but `ModelContextLimits::default()` is the legacy table with
      display names — `Claude (200K)`, `GPT-4o (128K)`, `GPT-4 Turbo (128K)`,
      `Gemini 1.5 Pro (1M)`. Exact-match-only would make the feature almost unusable
      from a shell (quoting, capitals, parentheses), so lookup needs to be forgiving —
      and forgiving lookup needs an explicit ambiguity rule, since `GPT-4` matches two
      entries. Designed below rather than discovered mid-implementation.
- [+] **B1 layering**: `--budget`'s clap `value_parser` runs before `AppPaths` exists
      and cannot read the override file, so the flag parses *syntactically* into a
      `BudgetSpec` and is resolved later in `settings`, where the file is reachable.
      A `value_parser` that reads the filesystem would also report a missing-model
      error as exit 2 (bad arguments) when it is really a resolution failure.
- [+] **B3 scoping**: "only the reports a review needs" is satisfied by the existing
      `ai_review` profile — `REVIEW_CHECKLIST_MD` is gated to `["full", "ai_review"]`,
      and key files / security scan / code quality / git-deep / refactoring are all in
      it. A *sixth report profile* would instead mean editing ~30 gating constants in
      `codepack-reports`, which is real new mechanics and touches artifact shape. Not
      done; the preset composes what exists, exactly as scoped.
- [+] `AI_PRESETS` is documented as "ported verbatim from legacy `constants.py`" and
      currently holds exactly five, pinned by `has_exactly_five_presets_in_legacy_order`.
      A sixth entry is an addition beyond legacy: the test must be updated to say so
      explicitly rather than just have its number bumped.

## B1 — `--budget` by model name

- [ ] `settings.rs`: `BudgetSpec { Tokens(u64), Model(String) }`; `parse_budget` keeps
      accepting `200000`/`200k`/`1M` and otherwise yields `Model`
- [ ] Resolution in `settings::resolve` against
      `ModelContextLimits::load_or_default(app_paths.model_limits_file())`, so a model
      added through the override file works without a rebuild — the mechanism's first
      real use
- [ ] Lookup rule, explicit: exact match, else case-insensitive exact, else
      case-insensitive substring **only when it matches exactly one** model. Ambiguous
      (`GPT-4` → two entries) is an error naming the candidates, never a silent pick.
      Unknown is an error listing what is available.
- [ ] `--budget 0` keeps meaning "no budget" (existing behaviour, must not regress)
- [ ] Tests: each accepted numeric form; exact/case-insensitive/substring hits;
      ambiguity is an error naming both candidates; unknown model is an error listing
      the available names; a model from an override file resolves

## B2 — `codepack explain <file>`

- [ ] `cli.rs`: `Explain(ExplainArgs)` — the file to explain, plus the shared project
      args so the same four configuration layers apply as for `preview`
- [ ] `commands/explain.rs`: build the plan exactly the way `preview` does (writing
      nothing, touching no history), then answer for one path
- [ ] Accepts an absolute path, a path relative to the project, or the
      backslash-joined form the plan itself stores; answers for whichever matches
- [ ] Three outcomes, all of them real answers: **included** (with group and severity),
      **excluded** (with the rule that excluded it, from `PlannedFile::reason`), and
      **not in the plan at all** — in which case say whether an ignored directory on
      its path explains it, because "your file is under `node_modules`" is the actual
      answer a user needs and the plan does not carry it per-file
- [ ] `--json` via the existing envelope (`command: "explain"`)
- [ ] Exit codes: 0 when an answer was produced (including "excluded" — that is a
      successful explanation, not a failure), 1 only for a real failure
- [ ] Tests: an included file; a file excluded by safe mode; a file under an ignored
      directory; a path that does not exist in the project at all; absolute and
      relative spellings of the same file agree

## B3 — a sixth preset, "PR Review"

- [ ] `presets.rs`: sixth entry — `export_profile: "ai_review"`,
      `diff_export_mode: "uncommitted"`, `include_git_patch: true`,
      `safe_export_mode: "balanced"` (matching its sibling `Code Review`, which serves
      the same audience; the two differ in *scope*, not in safety)
- [ ] Description in Russian, matching every other entry in that table
- [ ] The "exactly five, in legacy order" test becomes "the five legacy presets in
      order, plus one addition beyond legacy", so the boundary stays visible
- [ ] Tests: the preset exists, applies the expected fields to a `Config`, and
      `--preset` accepts it end to end

## Verification (at the very end, all three together)

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo xtask gate` (full: fmt, clippy, tests, `cargo deny check`, frontend
      format/typecheck/lint, `scripts/` suite, `sync-agents --check`, network isolation)
- [ ] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment
- [ ] Independent review pass (`codepack-quality-reviewer`) before merge

## Completion

- [ ] `docs/architecture/overview.md` — `codepack-cli` and `codepack-core` rows
- [ ] `docs/decisions/open-questions.md` — record anything this surfaces
- [ ] Checklist filled `+`/`-` honestly, final report in Russian
- [ ] Push to `origin`, fast-forward merge into `main`, delete every branch but `main`

---

## Next task

Not yet chosen. Start with the orientation ritual from
`.ai/project/13-progress-tracking.md`.
