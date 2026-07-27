# Task Checklist

**Task:** Raise the desktop frontend (`apps/desktop/ui`) from a functional prototype to a
professional-quality product interface: a real design system, a real desktop app shell,
reworked pages, adaptive layout, accessibility, and the settings that are currently dead.
**Date:** 2026-07-27
**Branch:** feat/frontend-redesign

Owner request: the frontend looks primitive, cluttered, inconvenient and non-adaptive.
The scope is the UI layer only — no domain crate changes, no artifact format changes,
no change to the Tauri command contract.

## Constraints that bound this task

- [ ] No CSS framework and no UI kit: CSP forbids remote sources, and
      `05-security-and-secrets.md` forbids a heavy dependency for a small need. The design
      system is hand-written CSS custom properties.
- [ ] `client.ts` stays the single choke point to the backend (S11 isolation invariant).
- [ ] Every new user-visible string exists in **both** `en.ts` and `ru.ts`.
- [ ] No new production dependency at all.

## Preparation

- [ ] Orientation ritual done (git, ROADMAP, overview, checklist, decisions)
- [ ] Current frontend read end to end; defects listed with evidence
- [ ] `pnpm install` so the frontend gate can actually run

## Implementation

### Design system

- [ ] `src/styles/tokens.css` — colour, spacing, radius, elevation, typography, motion
      scales for light and dark, replacing the 10 ad-hoc variables
- [ ] `src/styles/base.css` — reset, element defaults, focus-visible, scrollbars,
      selection, reduced-motion
- [ ] `src/styles/components.css` — buttons, fields, cards, chips, badges, tables,
      callouts, stats: the vocabulary every page shares

### App shell

- [ ] Sidebar navigation replacing the flat 8-button row: numbered wizard steps plus a
      separate "insights" group, with reachable/current/done state and a reason shown for
      a locked step instead of a silently disabled button
- [ ] Header with the active project, theme toggle and language toggle
- [ ] Status bar: version, the local-only privacy statement, watch state

### Shared components

- [ ] `Icon`, `Card`, `Field`, `Switch`, `Segmented`, `Stat`, `Callout`, `EmptyState`,
      `Toasts`, `StepProgress`

### Pages (all eight)

- [ ] Project — real first-run state, folder drag & drop, resolution trace shown
- [ ] Settings — grouped sections with descriptions instead of one flat column
- [ ] Security — summary stats, severity/kind filters, search, grouping by file
- [ ] Preview — stat bar, search, tree with size and hover actions, overrides panel
- [ ] Export (was "Log") — the eight pipeline steps rendered from `step`/`step_finished`,
      which the UI currently receives and throws away; elapsed time; log tools
- [ ] Result — outcome hero, stat cards, artifact list, grouped actions
- [ ] History — table, relative time, status pills, filters
- [ ] Analytics — stat cards, risk meter, stack chips

### Dead settings, wired up

- [ ] `ui_zoom` actually calls `set_ui_zoom` (today the slider changes nothing)
- [ ] `watch_enabled` actually calls `start_watch`/`stop_watch` and surfaces changes

### Adaptive + accessible

- [ ] Three layout breakpoints; nothing overflows at the 880px minimum window width or at
      200% UI zoom
- [ ] `aria-current`, live regions for progress and toasts, focus-visible everywhere,
      `prefers-reduced-motion` honoured

## Verification

- [ ] `pnpm format` / `typecheck` / `lint` / `build` clean
- [ ] `cargo xtask gate` green
- [ ] App actually run and inspected: light and dark, both languages, narrow width, and a
      real export from folder pick to result
- [ ] Independent review of the diff

## Completion

- [ ] `docs/architecture/overview.md` updated where the frontend's shape changed
- [ ] `ROADMAP.md` S11 status amended honestly
- [ ] `docs/decisions/open-questions.md`: the no-framework design-system decision
- [ ] Checklist filled `+`/`-`, final report in Russian
