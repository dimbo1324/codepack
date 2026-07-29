# Claude Code Project Configuration

Project-scoped Claude Code configuration for codepack.

Durable rules live in the shared modules under `.ai/` (universal + project).
`CLAUDE.md` imports them natively via `@` syntax. Codex consumes the same modules
through the generated `AGENTS.md` (regenerate with `cargo xtask sync-agents`; never edit
it by hand). Subagents should read `AGENTS.md` — it is the compiled single-file ruleset.

## Files

- `settings.json` — permission allowlist for routine read and verification commands,
  and an explicit denylist for destructive git operations and crate publishing.
- `agents/` — project-scoped subagents for focused delegation; name-for-name mirror of
  `.codex/agents/`.
- `skills/` — reusable project workflows; mirror of `.codex/skills/`.

## Recommended delegation

Do task-owning work on the main thread; spawn a subagent only for independent work that
does not need the main thread's full context:

- `codepack-stage-planner` — scope a `docs/__arch__/ROADMAP.md` stage before any code is written:
  boundaries, parity requirements, risks, acceptance criteria.
- `codepack-core-engine` — core crates: types, scanner, diff, storage, tokens, archive,
  orchestrator.
- `codepack-security` — the security crate: safety modes, redaction, detector.
- `codepack-reports` — reports and AI context packs.
- `codepack-desktop-ui` — Tauri shell and frontend.
- `codepack-quality-reviewer` — review a diff before finalizing it.
- `codepack-ci-triage` — debug a failing local or CI check.
- `codepack-repo-maintainer` — formatting, docs upkeep, rule sync, explicit publishing.

## Quality shortcuts

```powershell
cargo xtask gate --quick
cargo xtask fmt
cargo xtask doctor
cargo xtask sync-agents --check
```

Push to `main` only when the owner explicitly asked for it in the current task.

## Evolving the rules

The rule set is expected to change as the project learns. The protocol — mandatory
triggers, what may change autonomously, what needs owner approval — is in
`.ai/universal/08-rules-evolution.md`. Every rule change is recorded in
`.ai/CHANGELOG.md`. Never weaken a rule to make the current task pass.
