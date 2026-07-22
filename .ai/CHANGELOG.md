# Rule Changes Changelog

History of changes to the AI assistant rule system (`.ai/`, `CLAUDE.md`, `AGENTS.md`,
`.claude/`, `.codex/`). Every rule change gets an entry here — see
`universal/08-rules-evolution.md` for the protocol.

Format: date, what changed, why, who decided. Newest first.

---

## 2026-07-22 — Rule system switched to English

**Changed.** All agent-facing configuration translated from Russian to English:
`.ai/` modules, `CLAUDE.md`, `.claude/` and `.codex/` workspaces, the sync script,
and the generated `AGENTS.md`. Essence markers renamed from `**Суть.**` to
`**Essence.**`.

**Why.** Owner decision. English is the working language of the toolchain and keeps the
compiled entry point roughly half the byte size, which removes the pressure on the
32 KiB Codex instruction budget.

**Effect.** All modules returned to `inline` tier; the `extended` tier mechanism is
retained for future growth but currently unused. Language policy recorded in
`project/10-project-map.md`.

**Decided by.** Owner.

---

## 2026-07-22 — Rules evolution module added

**Changed.** New module `universal/08-rules-evolution.md` plus this changelog.

**Why.** Owner asked for the rule set to evolve during the project so agents always work
from current instructions. Without a defined protocol, rules drift out of sync with the
code and agents silently follow stale guidance.

**Effect.** Rule changes now have mandatory triggers, an autonomous-versus-approval
split, a fixed procedure, a review cadence, and a retirement path. The prime safeguard
forbids weakening a rule to make the current task pass.

**Decided by.** Owner.

---

## 2026-07-22 — Rule system created

**Changed.** Initial rule system: seven universal modules (workflow, checklist, scope
and style, architecture boundaries, security, quality, multi-assistant), five project
modules (project map, commands, domain rules, progress tracking, legacy reference),
`CLAUDE.md` entry point, generated `AGENTS.md`, mirrored `.claude/` and `.codex/`
workspaces with eight agents and five skills.

**Why.** The project is rewritten on Rust + Tauri almost entirely by AI agents. Agents
lose context between sessions, so the operating knowledge must live in files with a
defined orientation ritual.

**Effect.** Any agent in any session can locate the project state from files alone.

**Decided by.** Owner.
