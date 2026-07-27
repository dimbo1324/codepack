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

- [+] No CSS framework and no UI kit: CSP forbids remote sources, and
      `05-security-and-secrets.md` forbids a heavy dependency for a small need. The design
      system is hand-written CSS custom properties.
- [+] `client.ts` stays the single choke point to the backend (S11 isolation invariant) —
      verified by review: `@tauri-apps/*` is imported there and nowhere else
- [+] Every new user-visible string exists in **both** `en.ts` and `ru.ts` (273 keys,
      parity enforced at compile time by `Record<TranslationKey, string>`)
- [+] No new production dependency at all; `package.json` and the lockfile untouched

## Preparation

- [+] Orientation ritual done (git, ROADMAP, overview, checklist, decisions)
- [+] Current frontend read end to end; defects listed with evidence
- [+] `pnpm install` so the frontend gate can actually run

## Implementation

### Design system

- [+] `src/styles/tokens.css` — colour, spacing, radius, elevation, typography, motion
      scales for light and dark, replacing the 10 ad-hoc variables
- [+] `src/styles/base.css` — reset, element defaults, focus-visible, scrollbars,
      selection, reduced-motion
- [+] `src/styles/components.css` — buttons, fields, cards, chips, badges, tables,
      callouts, stats: the vocabulary every page shares

### App shell

- [+] Sidebar navigation replacing the flat 8-button row: numbered wizard steps plus a
      separate "insights" group, with reachable/current/done state and a reason shown for
      a locked step instead of a silently disabled button
- [+] Header with the active project, theme toggle and language toggle
- [+] Status bar: version, the local-only privacy statement, watch state

### Shared components

- [+] `Icon`, `Field`, `Switch`, `Segmented`, `Stat`, `Callout`, `EmptyState`, `Toasts`,
      `StepProgress`, `Sidebar`, `TopBar`, `StatusBar`
- [-] `Card` was listed in the plan and is **not** a component: cards are a global
      `.card` class. A wrapper adding nothing but a `<div>` would be ceremony, so the
      class stayed and the plan line was wrong, not the code.

### Pages (all eight)

- [+] Project — first-run state, folder drag & drop, resolution trace shown
- [+] Settings — grouped sections with descriptions instead of one flat column
- [+] Security — summary stats, severity filter, search, grouping by file
- [+] Preview — stat bar, search, tree with size and hover actions, overrides panel
- [+] Export (was "Log") — the pipeline steps rendered from `step`/`step_finished`,
      which the UI previously received and threw away; elapsed time; log tools
- [+] Result — outcome hero, stat cards, artifact list, grouped report actions
- [+] History — table, relative time, status pills, scope filter
- [+] Analytics — stat cards, risk meter, stack chips

### Dead settings, wired up

- [+] `ui_zoom` calls `set_ui_zoom` live, and the stored zoom is applied at startup
- [+] `watch_enabled` calls `start_watch`/`stop_watch`, surfaces changes in the status
      bar and as a toast, and stops when the project changes

### Adaptive + accessible

- [+] Three layout breakpoints; no horizontal overflow on any of the eight pages at
      1280 / 880 / 600 CSS px
- [+] `aria-current`, live regions for progress and toasts, focus-visible everywhere,
      `prefers-reduced-motion` honoured, radiogroup keyboard contract (roving tabindex +
      arrow keys) on `Segmented`

## Verification

- [+] `pnpm format` / `typecheck` (134 files, 0/0) / `lint` / `build` clean
- [+] `cargo xtask gate` green; `cargo test --workspace` 938 passed / 0 failed
- [+] App driven end to end against a stubbed IPC layer in a browser — all eight pages,
      light and dark, RU and EN, three widths, a full export replayed from events
- [-] **Not** run as the native Tauri window. Screenshots of a native window are outside
      what this environment can capture, so verification used the real frontend bundle
      against stubbed `invoke`/event responses. What that cannot prove: the OS-level
      appearance, and that `getCurrentWebview().onDragDropEvent` fires under the real
      capability set (it needs no permission beyond `core:default`, which the webview
      already has, but that reasoning is not the same as observing it).
- [+] Independent review of the diff (`codepack-quality-reviewer`), 16 findings; all
      correctness, i18n and dead-code findings fixed, see below

## Findings from the review, and what happened to them

- [+] Progress events emitted before `start_export` returned were dropped — now buffered
      by run id and replayed on adoption
- [+] `ParsedStep.total` was parsed and ignored; the denominator is now the engine's own
- [+] `AnalyticsPage` had an unguarded async `$effect` — generation counter added
- [+] Settings selects desynced from the config when an apply failed — rolled back
- [+] Startup failure rendered `[object Object]` — `errorMessage` extracts `CommandError`
- [+] Untranslated backend enums on screen — `i18n/enums.ts`, falling back to the raw
      value so an unknown one is reported rather than invented
- [+] `role="radiogroup"` without the keyboard contract — implemented
- [+] Fragile `{#each}` key, unbounded export log (now capped at 5000 lines with the
      dropped count shown), 13 dead i18n keys, 11 dead CSS classes — all fixed; a script
      now confirms zero unreachable keys and zero unreferenced global classes
- [+] Toasts stored translated strings and froze across a language switch — they store
      keys now
- [+] Two documentation claims corrected in `ROADMAP.md`

## Completion

- [+] `docs/architecture/overview.md` updated (new frontend row)
- [+] `ROADMAP.md` S11 status amended with an honest account, including what was not done
- [+] `docs/decisions/open-questions.md`: the no-framework design-system decision and the
      drag-and-drop / capabilities decision
- [+] Checklist filled `+`/`-`, final report in Russian
