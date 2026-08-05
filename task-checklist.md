# Task Checklist

**Task:** Full-precision timestamps everywhere a date is read or shown — hours, minutes
and seconds, especially for Git commits.

**Date:** 2026-08-05
**Branch:** feat/full-precision-timestamps

Owner instruction, 2026-08-05: whenever information carrying a date is taken, the
precise moment must come with it — hours, minutes, seconds. Git commit inspection is
the named case. Owner approved, once green: merge into `main` and push.

## Preparation

- [ ] Orientation ritual: git status/log, ROADMAP, overview, previous checklist,
      open questions
- [ ] Inventory every place a date is produced or displayed, and separate the ones that
      already carry seconds from the ones that truncate

## Implementation — product

- [ ] `codepack-core::time`: retire the date-only formatter so a coarse rendering has no
      home in the workspace
- [ ] `codepack-reports` Git timeline: commit lines carry the full UTC moment instead of
      `YYYY-MM-DD`
- [ ] `codepack-engine` Git report: the recent-log section carries each commit's moment
      instead of an `--oneline` listing with no time at all
- [ ] Desktop history table: the exact moment is the visible value; relative time moves
      to the tooltip

## Implementation — agent rules

- [ ] New universal rule module for timestamp precision, extended tier so the generated
      `AGENTS.md` stays inside its size budget
- [ ] Orientation ritual and multi-assistant module: the git-log commands they prescribe
      show committer dates
- [ ] `.claude/` and `.codex/` mirrors updated in the same task
- [ ] `.ai/CHANGELOG.md` entry; `AGENTS.md` regenerated

## Documentation

- [ ] Owner decision recorded in `docs/__arch__/open-questions.md`, with the reasoning on
      why invariant I5 does not require a `schema_version` bump here
- [ ] `docs/architecture/overview.md` refreshed if the shape changed

## Verification

- [ ] `cargo xtask fmt`
- [ ] `cargo xtask gate` — full gate, all sections
- [ ] Self-review of the diff

## Completion

- [ ] Checklist filled with `+`/`-`
- [ ] Merge into `main` fast-forward, push to `origin/main`
- [ ] Final report
- [ ] Shut down the PC (explicit owner instruction)
