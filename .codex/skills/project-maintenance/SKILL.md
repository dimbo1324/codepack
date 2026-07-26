---
name: project-maintenance
description: Use for routine codepack upkeep — formatting, quality gates, rule sync, mirror consistency, state-document updates, and explicitly requested publishing.
---

# Repository Maintenance

Use the project's automation instead of hand-rolled command sequences.

## Fast paths

The orchestrator is the door to routine work; it wraps the `xtask` commands rather than
reimplementing them, so both reach the same code.

```powershell
python dev_tools_scripts_runner.py list        # the catalog, machine-readable
python dev_tools_scripts_runner.py doctor      # what this host can and cannot do
python dev_tools_scripts_runner.py format-code # rustfmt + Prettier
python dev_tools_scripts_runner.py quality-gate
python dev_tools_scripts_runner.py selftest    # after touching anything under scripts/
```

Reach for `xtask` directly when you need a step the catalog does not name:

```powershell
cargo xtask gate --quick          # quick local gate
cargo xtask sync-agents           # regenerate AGENTS.md from .ai/
cargo xtask sync-agents --check   # verify it is in sync
```

## Rules

- `AGENTS.md` is **generated** and never hand-edited. Edit the module in `.ai/`, then
  run the sync command.
- The `AGENTS.md` budget is 30 KiB. If the build hits the limit: tighten a module, or
  mark a situational one with `<!-- tier: extended -->` and give it an
  `> **Essence.**` line.
- Rule changes follow the `rules-evolution` skill and
  `.ai/universal/08-rules-evolution.md`, including the `.ai/CHANGELOG.md` entry.
- `.claude/agents|skills` and `.codex/agents|skills` are name-for-name mirrors. Changing
  one side requires the equivalent change on the other **in the same task**.
- `.claude/settings.json`: extending the allowlist is fine; removing a deny entry needs
  explicit owner approval.
- **The scripts are infrastructure, so keeping them true is part of the task that changed
  the workflow, not a follow-up.** If a task changes how the project is built, checked,
  formatted, run or cleaned, update the matching script in that same task and run
  `selftest`. A script that describes a workflow which no longer exists is worse than no
  script, because someone will trust it. New routine work gets a new directory under
  `scripts/` plus one catalog entry — never new Python in `scripts/runner/`.
- Keep them **cross-platform**: resolve tools through `_toolkit/processes.py`, and let
  genuinely Windows-only work decline with a reason instead of failing part-way.

## State documents

- Stage completed → `**Status.**` line under that stage in `ROADMAP.md` (in Russian, to
  match the file) plus the §1 table.
- System shape changed → `docs/architecture/overview.md`.
- Owner decision → `docs/decisions/open-questions.md`.
- New invariant → `docs/architecture/invariants.md`.
- Task closed → `task-checklist.md` with honest `+`/`-` marks.

## Publishing

Push to `main` only when the owner explicitly asked in the current task. The default
route is branch → gate → fast-forward merge → report. A red gate forbids publishing.
