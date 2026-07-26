# Project: codepack

A cross-platform desktop application that turns a source folder into a **clean, safe
snapshot**: an archive plus a set of reports fit to hand to an AI assistant **and to a
human** (developer, team, junior, or a non-programmer technical stakeholder).

The core value is **preventing secret leakage** when a project is shared outward. This
is not an archiver and not a README generator: the product is about safe context
handoff with local-only analysis.

## Repository map

Rust workspace under `crates/`:

- `codepack-core` — domain types, configuration, errors, progress and cancellation.
- `codepack-scanner` — tree walking, ignore rules, stack detection, export planning.
- `codepack-security` — export safety modes, secret redaction, the detector.
- `codepack-diff` — differential export and snapshots (via `git2`).
- `codepack-storage` — SQLite: history, snapshots, findings, migrations.
- `codepack-tokens` — bytes and tokens, budget mode.
- `codepack-reports` — insight reports, AI context packs, dashboard.
- `codepack-archive` — ZIP building, splitting, restore.
- `codepack-engine` — orchestrator of the eight-step pipeline.
- `codepack-cli` — headless binary.
- `xtask` — the project's task runner and quality gate.

Other areas:

- `apps/desktop/ui` — Svelte + Vite + TypeScript frontend (pnpm workspace).
- `apps/desktop/src-tauri` — the Tauri shell (crate `codepack-desktop`, binary
  `codepack-desktop`). It is a member of the same cargo workspace as `crates/*`, and it calls
  `codepack-engine` directly rather than shelling out to `codepack-cli`: the two front
  ends sit side by side over the engine (BLUEPRINT §C.2), not in a chain.
  The webview holds **no filesystem permission** (`capabilities/default.json`); every
  file operation is a `#[tauri::command]`, and the frontend's only route to the backend
  is `ui/src/lib/api/client.ts`.
- `docs/` — state documents and decisions; `docs/__arch__/` — legacy archive.
- `.ai/`, `.claude/`, `.codex/` — assistant rules and workspaces.

The product intent lives in `BLUEPRINT.md`; the plan and progress live in `ROADMAP.md`.
The full map of state documents is in the progress-tracking module.

## Language policy

- **Agent-facing infrastructure is English**: `.ai/`, `.claude/`, `.codex/`,
  `CLAUDE.md`, `AGENTS.md`, `docs/` state documents, `task-checklist.md`, code, code
  comments, commit messages, and test names.
- **Owner-facing product documents are Russian**: `BLUEPRINT.md`, `ROADMAP.md`,
  `README.md`. When you add a `**Status.**` line to `ROADMAP.md`, write it in Russian
  to match the file.
- **Reports to the owner are Russian.**

Do not mix languages inside a single file.

## Documentation policy

- `BLUEPRINT.md` is the product specification; it changes only when the product intent
  changes, and only by owner agreement.
- `ROADMAP.md` is the plan and progress record; update it when a stage completes.
- New documents are created only on direct request. Exception: `docs/architecture/` and
  `ROADMAP.md` must stay accurate when architecture or progress changes.

## Product guardrails

- **Privacy is absolute.** Analysis is local; network access is forbidden everywhere
  except the explicitly user-initiated AI handoff in stage S13.
- **The source is immutable.** Export never writes into the source project folder.
- **Bytes stay.** Byte-based size reporting is preserved everywhere it existed; tokens
  are an addition, never a replacement (owner decision).
- **Parity before novelty.** Within a stage, reproduce the legacy behavior first, then
  add new capability.
- **Stage order is binding** (`ROADMAP.md` §1, S0→S14). Skipping ahead requires an owner
  decision recorded in `docs/decisions/open-questions.md`.
- **Artifact formats are backward compatible**; changing one requires bumping
  `schema_version`.
