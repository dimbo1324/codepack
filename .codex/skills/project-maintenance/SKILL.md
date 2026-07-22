---
name: project-maintenance
description: Use for routine codepack upkeep — formatting, quality gates, rule sync, mirror consistency, state-document updates, and explicitly requested publishing.
---

# Repository Maintenance

Use the project's automation instead of hand-rolled command sequences.

## Fast paths

```powershell
cargo xtask fmt                   # format Rust sources
cargo xtask gate --quick          # quick local gate
cargo xtask gate                  # full gate before merging
cargo xtask doctor                # read-only environment diagnostics
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
