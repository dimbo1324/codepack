# Rule Changes Changelog

History of changes to the AI assistant rule system (`.ai/`, `CLAUDE.md`, `AGENTS.md`,
`.claude/`, `.codex/`). Every rule change gets an entry here — see
`universal/08-rules-evolution.md` for the protocol.

Format: date, what changed, why, who decided. Newest first.

---

### 2026-07-26 — Windows-only, форматирование, установщик: модули команд и карты проекта

**Что.** `.ai/project/11-commands.md`: платформенные заметки переписаны под
единственную поддерживаемую платформу (Windows 10/11) с указанием, что отключённый
кроссплатформенный код помечен `TODO(cross-platform)`, а не удалён; добавлены команды
`cargo xtask install-hooks` и `cargo xtask package`; добавлен раздел «Formatting»
(rustfmt + Prettier, `pre-commit`-хук через `core.hooksPath`); гейт-политика получила
фронтенд-проверки и правило «в CI они падают, а не пропускаются»; исправлена команда
запуска в разработке — `pnpm desktop:dev` вместо `pnpm --filter @codepack/ui exec tauri
dev`, которая никогда не работала. `.ai/project/10-project-map.md`: имя бинаря
десктопа исправлено на `codepack-desktop` (в файле стояло `codepack`, что фактически
неверно с S11). Зеркала `.claude/agents/codepack-desktop-ui.md` и
`.codex/agents/desktop-ui.toml`: `cargo tauri dev` → `pnpm desktop:dev` в обоих.

**Почему.** Решение владельца 2026-07-26 сузило область сборки до Windows и потребовало
автоформатирования и установщика — то есть изменились и платформа, и набор команд, и
гейт. Плюс три факта в модулях были попросту ложными: команда `tauri dev`, имя бинаря и
обещание, что фронтенд-проверки выполняются в CI (в воркфлоу не было ни Node, ни pnpm,
поэтому они молча пропускались).

**Кто решил.** Владелец — сужение области, форматирование, установщик. Ложные факты и
пропущенная запись в этом файле — находки независимого ревью
(`codepack-quality-reviewer`); пропуск записи в CHANGELOG оказался повтором того же
класса дефекта, что и запись 2026-07-25 ниже.

### 2026-07-25 — `cargo xtask golden` добавлена в модуль команд

**Что.** `.ai/project/11-commands.md` получил команду `cargo xtask golden` и абзац о
том, что это команда для машины разработчика: ей нужен Python, CI её никогда не
запускает, а перегенерировать эталоны ради зелёного теста запрещено.

**Почему.** Команда появилась вместе с golden-паритетом (`ROADMAP.md` §8), но модуль
правил о ней не знал. `sync-agents --check` такой пробел не ловит по построению —
он проверяет синхронность `AGENTS.md` с модулями, а не полноту самих модулей.
Прецедент ровно тот же, что с `cargo xtask deny` 2026-07-22.

**Кто решил.** Найдено независимым ревью core-hardening-задачи.


## 2026-07-22 — `cargo xtask deny` documented

**Changed.** `.ai/project/11-commands.md` now lists `cargo xtask deny` and notes that
`cargo-deny` is a separately-installed binary, not a `rust-toolchain.toml` component.

**Why.** Stage S1 wired `cargo deny check` into `cargo xtask gate` now that the
workspace has real dependencies (`deny.toml`'s own stated trigger condition).

**Effect.** Contributors and CI both need `cargo-deny` on `PATH` for the gate to pass;
`cargo xtask doctor` reports its presence.

**Decided by.** Owner (S1 task).

---

## 2026-07-22 — Sync tooling moved to `cargo xtask`; two modules marked `extended`

**Changed.** `dev_tools_scripts_runner.py` and `scripts/dev_tools/sync_agents_md.py`
removed; `cargo xtask sync-agents` (already implemented) is now the only way to
regenerate `AGENTS.md`. References in `.ai/README.md` and `.claude/settings.json`
updated. `universal/08-rules-evolution.md` and `project/14-legacy-reference.md` marked
`<!-- tier: extended -->` with an `> **Essence.**` line each.

**Why.** Stage S0 (`ROADMAP.md` §2) requires the temporary Python sync script to be
retired in favor of `cargo xtask`. Separately, module growth since the previous entry
pushed the assembled `AGENTS.md` to 31.7 KiB, over the 30 KiB budget — `sync-agents
--check` was failing.

**Effect.** `cargo xtask sync-agents --check` passes again (25.7 KiB). Rules-evolution
and legacy-reference are situational (read when a task actually touches them), so they
compress well; `CLAUDE.md` still imports them in full for Claude Code regardless of tier.

**Decided by.** Agent, within the autonomous-fix scope of `08-rules-evolution.md`
(correcting a stale reference and restoring a budget invariant, not loosening a rule).

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
