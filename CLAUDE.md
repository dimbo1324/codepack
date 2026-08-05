# codepack — working notes for Claude Code

This file is the Claude Code entry point. All rules live in shared modules under `.ai/`
— the single source of truth for every AI assistant in this repo. Codex reads the same
modules through the generated `AGENTS.md`; never edit that file by hand (edit a module,
then run `cargo xtask sync-agents`).

Later modules override earlier ones; an explicit owner instruction in the current
conversation overrides everything.

## Starting a fresh session

This project is built almost entirely by AI agents, so sessions often begin with no
context. Before any work, run the **orientation ritual** from
`@.ai/project/13-progress-tracking.md`: git status and log → `docs/__arch__/ROADMAP.md` (the first
stage without a `**Status.**` line is next) → `docs/architecture/overview.md` →
`task-checklist.md` → `docs/__arch__/open-questions.md`.

The product intent is described in full in `docs/__arch__/BLUEPRINT.md`. The legacy Python
implementation is archived at `docs/__arch__/codepack-main.zip` and serves as the
behavioral reference.

**Routine jobs go through the script orchestrator**, not hand-assembled commands:
`python dev_tools_scripts_runner.py list` prints the catalog, `... <name>` runs one.
It is cross-platform and it is the same door humans and agents use, so a workflow that
changed is visible to everyone at once. Details, and the standing duty to keep the
scripts current, are in `@.ai/project/11-commands.md`.

The rules themselves are meant to evolve: see `@.ai/universal/08-rules-evolution.md`
for when and how to change them, and `.ai/CHANGELOG.md` for what changed so far.

## Universal rules (portable to any project)

- @.ai/universal/01-workflow.md
- @.ai/universal/02-task-checklist.md
- @.ai/universal/03-scope-and-code-style.md
- @.ai/universal/04-architecture-boundaries.md
- @.ai/universal/05-security-and-secrets.md
- @.ai/universal/06-quality-and-testing.md
- @.ai/universal/07-multi-assistant.md
- @.ai/universal/08-rules-evolution.md
- @.ai/universal/09-time-and-timestamps.md

## Project rules (codepack)

- @.ai/project/10-project-map.md
- @.ai/project/11-commands.md
- @.ai/project/12-domain-rules.md
- @.ai/project/13-progress-tracking.md
- @.ai/project/14-legacy-reference.md
- @.ai/project/15-command-reference.md

## Claude Code workspace

- `.claude/settings.json` — permission allow and deny lists.
- `.claude/agents/` — project subagents (`codepack-*`) for delegated work; mirrors
  `.codex/agents/` one-to-one.
- `.claude/skills/` — reusable workflows (`stage-episode`, `legacy-lookup`,
  `code-review`, `ci-fix`, `project-maintenance`, `rules-evolution`); mirrors
  `.codex/skills/`.
