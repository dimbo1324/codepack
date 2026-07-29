# Codex Project Configuration

Project-scoped Codex configuration for codepack.

Rules live in the shared modules under `.ai/`. Codex reads them through the generated
`AGENTS.md` in the repository root — the compiled single-file ruleset. Claude Code gets
the same rules through `CLAUDE.md` with native `@` imports.

`AGENTS.md` is **generated** and never hand-edited. After changing any `.ai/` module:

```powershell
cargo xtask sync-agents
```

## Files

- `config.toml` — project-scoped model and agent defaults. No secrets belong here.
- `agents/` — project agents; name-for-name mirror of `.claude/agents/`.
- `skills/` — reusable workflows; mirror of `.claude/skills/`.

## Agents

- `codepack-stage-planner` — scope a `docs/__arch__/ROADMAP.md` stage before any code is written.
- `codepack-core-engine` — core crates: types, scanner, diff, storage, tokens, archive,
  engine.
- `codepack-security` — safety modes, redaction, detector.
- `codepack-reports` — reports, AI context packs, dashboard.
- `codepack-desktop-ui` — Tauri shell and frontend.
- `codepack-quality-reviewer` — review a diff before finalizing.
- `codepack-ci-triage` — debug a failing check.
- `codepack-repo-maintainer` — formatting, state documents, rule sync, publishing.

## Skills

`stage-episode`, `rules-evolution`, `legacy-lookup`, `code-review`, `ci-fix`,
`project-maintenance`.

## Mirroring

`.codex/agents|skills` and `.claude/agents|skills` are name-for-name mirrors. Changing
one side requires the equivalent change on the other in the same task.

## Evolving the rules

The rule set is expected to change as the project learns. The protocol lives in
`.ai/universal/08-rules-evolution.md`; every change is recorded in `.ai/CHANGELOG.md`.
Never weaken a rule to make the current task pass.
