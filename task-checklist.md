# Task Checklist

**Task:** A cross-platform Python dev-tools orchestrator, plus the legacy application
icon adopted into the current project.
**Date:** 2026-07-26
**Branch:** feat/python-dev-orchestrator-and-legacy-icon

Two owner-requested tasks. The reference implementation the owner pointed at
(`country-decision-atlas-r/dev_tools_scripts_runner.py`) was read end to end first: a
nine-line root shim, a package holding all logic, and a hand-edited JSON catalog that the
loader validates before anything can launch.

## The one tension to name up front

The orchestrator must be **cross-platform** ("на любой операционной системе"), while the
product itself was narrowed to **Windows 10/11 only** yesterday. These do not conflict, and
the resolution is deliberate: every script *runs* anywhere, and the ones whose work is
inherently Windows-specific (building the `.exe` installer) detect the host and refuse with
a clear message naming the owner decision, rather than failing cryptically half-way through
a Rust build. A script that lies about what it can do on the current OS is worse than one
that declines.

## Architecture (adapted from the reference, not copied)

- `dev_tools_scripts_runner.py` at the repository root — a thin shim, nothing else.
- `scripts/runner/` — the orchestrator's own logic, mirroring the reference's separation:
  `models` (pure data) / `exceptions` / `config_loader` (validates) / `registry` (queries) /
  `execution` (subprocess) / `rendering` (prints) / `interactive` (reads input) / `main`
  (dispatch). Nothing prints and reads input in the same module.
- `scripts/runner/config/*.json` — the hand-edited catalog: `meta`, `categories`,
  `cadences`, `scripts`.
- One directory per script, each with its own `config/*.json`. Small scripts stay one
  module; big ones decompose fully.
- **Scripts are launched as modules** (`python -m scripts.<name>`), not by file path. This
  is the one deliberate divergence from the reference: it makes small and large scripts
  launch identically, lets a decomposed script use ordinary relative imports, and needs no
  `sys.path` manipulation. The loader validates that a declared module sits under
  `scripts.` — the equivalent of the reference's escape-the-root check.
- No behaviour hardcoded in Python: each script reads its own JSON.

## Preparation

- [ ] Reference orchestrator studied: entry shim, config validation, registry, execution,
      rendering, interactive shell, and all four config files
- [ ] Baseline: `main` green (CI run on `1038f2b` all 14 steps `success`), 911 tests
- [ ] Checklist committed before any code

## Task 1a — the orchestrator core

- [ ] `dev_tools_scripts_runner.py` — shim only, imports and calls `main`
- [ ] `scripts/runner/` modules as listed above
- [ ] Config validation turns a bad hand-edit into one clear message: missing file, bad
      JSON, unknown category reference, duplicate identifier, module outside `scripts.`
- [ ] Interactive menu, direct invocation (`... clean`), and `help` all work
- [ ] Non-interactive with no arguments runs the default script instead of blocking on
      `input()` — the reference's behaviour, and the one that matters for agents and CI
- [ ] RU/EN interface, English default (matches the project's language policy for tooling)

## Task 1b — the scripts

- [ ] `doctor` — report which tools are present (cargo, rustc, node, pnpm, cargo-deny,
      git, python) and what the host OS means for the other scripts
- [ ] `format_code` — rustfmt + Prettier, the same work `cargo xtask fmt` does
- [ ] `quality_gate` — the full gate; the default script
- [ ] `clean_project` — **the destructive one, so the most decomposed.** Removes what git
      does not track, plus empty directories. Dry-run is the default; deleting requires an
      explicit flag or confirmation. Never touches ignored-but-precious paths listed in its
      own config
- [ ] `build_installer` — the Windows `.exe`; declines clearly on other hosts
- [ ] `dev_run` — build, launch for manual testing, and clean up after the window closes,
      with the cleanup level set in config
- [ ] `install_hooks` — the formatting pre-commit hook
- [ ] Every script: `--help` works, exit codes are meaningful, no hardcoded parameters

## Task 2 — the legacy icon

- [ ] Extract `assets/ICO.ico` from `docs/__arch__/codepack-main.zip` (128×128, 32-bit,
      single image) into a temporary directory outside the repository, per the legacy
      reference rules
- [ ] Produce a proper icon set with `tauri icon` rather than hand-rolling sizes: the
      current `icon.png` is a 2.2 KB placeholder and the current `.ico` is unrelated
- [ ] Verify the generated `.ico` really carries multiple sizes — a single 128×128 scaled
      down is blurry in the taskbar, which is where a user actually sees it
- [ ] Confirm the icon reaches both places it matters: the installer/executable, and
      `default_window_icon()`, which is what our tray icon uses

## Verification

- [ ] `cargo xtask gate` green
- [ ] Every script executed for real, not just imported: `doctor`, `format_code`,
      `quality_gate`, `clean_project --dry-run`, `install_hooks`, `build_installer`
- [ ] `clean_project` proven safe on a scratch copy before it is ever pointed at the repo
- [ ] The orchestrator's config validation proven by feeding it a deliberately broken edit
- [ ] Installer rebuilt with the new icon and the icon confirmed in the artifact
- [ ] Independent review of the diff
- [ ] CI green on `windows-latest`

## Completion

- [ ] `.ai/project/11-commands.md` documents the orchestrator and the duty to keep the
      scripts current and cross-platform
- [ ] `CLAUDE.md`, `AGENTS.md` (regenerated), `.claude/`, `.codex/` all point agents at it
- [ ] `.ai/CHANGELOG.md` entry for the rule-module change
- [ ] `docs/architecture/overview.md` and `docs/decisions/open-questions.md` updated
- [ ] Checklist filled `+`/`-`, final report in Russian
