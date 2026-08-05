---
name: stage-episode
description: Use to plan and execute one ROADMAP stage (S0–S14) end to end — orientation, scoping, parity-first implementation, verification, and status update.
---

# Executing a Roadmap Stage

One `docs/__arch__/ROADMAP.md` stage is one task. Do not merge two stages into one task, and do not
skip the S0→S14 order without an owner decision.

## 1. Orientation (mandatory, no skipping)

```powershell
git status --short --branch
git log -15 --date=iso-strict --pretty=format:"%h %cd %s"
```

Then read, in order: `docs/__arch__/ROADMAP.md` §1 and the `**Status.**` lines (the first stage
without one is yours), `docs/architecture/overview.md`, `task-checklist.md`,
`docs/__arch__/open-questions.md`.

If `task-checklist.md` still has open `[ ]` items from a previous session, resolve them
first.

## 2. Planning

For a large or unfamiliar stage, delegate to the `codepack-stage-planner` subagent.

Define: stage boundaries, the parity required against the legacy version, which 🎯 items
belong here, risks to invariants, and acceptance criteria.

Fill `task-checklist.md` with `[ ]` items grouped into preparation / implementation /
verification / completion, and **commit it before writing code**.

## 3. Branch

```powershell
git checkout main
git pull --ff-only origin main
git checkout -b feat/s<N>-short-description
```

## 4. Implementation — parity before novelty

Reproduce the legacy behavior first, then add new capability. Facts come from
`docs/__arch__/BLUEPRINT.md`; when literal precision is needed, use the `legacy-lookup` skill.

Delegate to the specialist subagents: `codepack-core-engine`, `codepack-security`,
`codepack-reports`, `codepack-desktop-ui`.

## 5. Verification

```powershell
python dev_tools_scripts_runner.py quality-gate
```

If the stage changed how the project is built, checked, formatted, run or cleaned, update
the matching script under `scripts/` in this same task and run `selftest`.

Run the `code-review` skill or the `codepack-quality-reviewer` subagent before
finalizing.

## 6. Completion

- Mark checklist items `+`/`-` honestly.
- Add the `**Status.**` line under the stage in `docs/__arch__/ROADMAP.md` (in Russian, matching that
  file) and update the §1 table.
- Update `docs/architecture/overview.md` if the system's shape changed.
- If the stage exposed stale or missing rules, apply the `rules-evolution` skill.
- Fast-forward merge into `main` only with a green gate.
- Write the final report: what was done, what was verified, what was not done.
