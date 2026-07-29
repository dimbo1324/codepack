# Architecture: what exists today

> This document describes what is **actually in the code**, not what is planned. It is
> updated whenever the shape of the system changes: a new crate, a new layer, a new
> operational job.
>
> Rewritten in English on 2026-07-30, when the documentation was split into internal and
> external sets. The per-date engineering history that used to live here is in the git
> log and in the internal plan; this file answers "what is built and how does it fit
> together".

**Last revised:** 2026-07-30
**Target platform:** Windows 10/11 only. macOS and Linux remain a product goal but are
switched off; the disabled cross-platform code is marked `TODO(cross-platform)` rather
than deleted, and lives in exactly one domain crate (`codepack-core::paths`).

## The shape of the system

```text
codepack-core            domain types, config, paths, cancellation, time, classification
   ↑
codepack-scanner  codepack-security  codepack-diff  codepack-storage  codepack-tokens
codepack-reports  codepack-archive   codepack-sanitize
   ↑
codepack-engine          the eight-step export pipeline
   ↑                ↑
codepack-cli      apps/desktop (Tauri + Svelte)
```

Dependencies point strictly downward and there are no cycles. The two front ends sit
**side by side** over the engine — the desktop app calls `codepack-engine` directly, it
does not shell out to the CLI. No `codepack-*` crate knows about Tauri or the frontend
(invariant I8), so the whole core builds and tests headless.

## The crates

| Crate | What it does |
|---|---|
| `codepack-core` | Domain types and `Config` (27 fields plus `schema_version`), normalization, migration from the legacy settings file, the six AI presets, `AppPaths`, `CancellationToken`, progress and log events. Also the single home for things that were once duplicated: text/binary classification (`classify`), the civil-date algorithm (`time`), and the `.codepack-allow` format and fingerprint recipe (`allowlist`). |
| `codepack-scanner` | Walks the tree, applies ignore rules (base, per-stack, `.exportignore`, user rules), detects the stack, and builds the export plan. Symlinks are never followed (invariant I7). |
| `codepack-security` | Safe-export modes, secret redaction, and the detector: provider signatures, entropy, a keyword cascade and named risky-code shapes. Carries an accuracy corpus test whose precision/recall thresholds may never be lowered (invariant I9). Emits SARIF 2.1.0. |
| `codepack-diff` | Differential export and snapshots through `git2`. Never requires a `git` binary. |
| `codepack-storage` | SQLite: seven tables plus `schema_version`, numbered migrations, run history, snapshots, findings, and per-project retention. Has no runtime dependency on any other `codepack-*` crate. |
| `codepack-tokens` | Byte formatting (preserved verbatim from the previous version — invariant I4), token estimation, the budget selection, and `ModelContextLimits`, the model→context-window table that `--budget <model>` resolves through. |
| `codepack-reports` | Around thirty insight reports, `PROJECT_PROFILE.json`, the AI context and prompt folders, the HTML dashboard, and the human-oriented reports (project overview, onboarding guide, review checklist). |
| `codepack-archive` | Archive building and restore. Two entry points: the export pipeline's planned, splittable, reported output, and `pack_files`, which packs a caller-named list of files into one archive. Both honour `ArchiveFormat` — ZIP by default, 7z on request, RAR reserved and refused. Extraction is path-traversal safe (invariant I7). |
| `codepack-sanitize` | The "sterile copy": comments stripped with real tree-sitter parsers (never regex) and code reformatted by whichever formatter is found on `PATH`. Reuses the scanner's file selection, the security crate's safety filter and its redaction — never a second, less guarded path out of the project. Optionally packs the result into one archive. |
| `codepack-engine` | The orchestrator: plan → copy → structure → git → text dump → analytics → manifest → archive. Cancellation is checked inside each step's loops, not only between steps. The only place `codepack_security::scan_project` is called in the pipeline. |
| `codepack-ai` | The stage S13 integration — the one and only crate permitted to reach the network. |
| `xtask` | The task runner and quality gate. |

## The two front ends

**`codepack-cli`** — the `codepack` binary. Nine commands: `export`, `preview`, `scan`,
`history`, `doctor`, `sanitize`, `completions`, `verify`, `explain`. Its published
contracts live in their own modules with their own tests, because other people's
pipelines depend on them:

- **Exit codes** `0` success, `1` error, `2` bad arguments, `3` critical secrets found.
  Code `2` is emitted deliberately rather than inherited from `clap`'s default, and a
  real failure always outranks "secrets found" — returning `3` for a run that broke
  would tell a pipeline the scan result can be trusted when it cannot.
- **`--json`** carries `schema_version` and a `command` discriminator, with the payload
  flattened. Machine output is the only thing on stdout; progress, warnings and errors
  go to stderr, or `codepack export --json | jq` would break on the first log line.
- **Configuration** resolves in four layers, narrower scope winning: defaults → global
  settings → `.codepack.toml` → flags. `--preset` sits between the project file and the
  flags, because a preset is a named bundle of flags and must not override one the user
  typed.

**`apps/desktop`** — the Tauri shell (`codepack-desktop`) and a Svelte 5 + TypeScript
frontend. The webview holds **no filesystem permission**: every file operation is a
`#[tauri::command]`, and the frontend's only route to the backend is one typed client
module. The content security policy admits no remote sources, so invariant I1 is held at
the webview level rather than by convention. Exports run on a background thread with a
run id and can be cancelled.

## Supporting parts

| Part | State |
|---|---|
| Project config (`.codepack.toml`) | TOML at the project root, all fields optional, overrides global settings. An unknown key is an error naming the key, not a silent no-op. |
| Golden parity (`tests/golden/`) | References produced by actually running the archived previous implementation, on three fixtures. Regenerated by `cargo xtask golden`; never edited to make a comparison pass. |
| Quality gate (`cargo xtask gate`) | Eight sections: format, clippy with warnings denied, tests, `cargo deny`, frontend format/typecheck/lint, the `scripts/` suite, agent-rule sync, and network isolation. |
| Network isolation | A gate step, not a convention: it reads every crate manifest and fails if an HTTP client appears anywhere but `codepack-ai`. |
| Dev scripts (`dev_tools_scripts_runner.py`, `scripts/`) | The cross-platform door to routine jobs — quality gate, formatting, dev run, installer, doctor, hooks, clean, selftest. |
| CI (`.github/workflows/ci.yml`) | `windows-latest` only; the other legs are commented out rather than deleted. |
| Packaging | `cargo xtask package` produces an NSIS installer. Signing, notarisation, checksums and auto-update are not done. |

## Known debt

- Artifact localization is still a pilot on a single report; the rest of the catalogue
  is English only.
- Archive splitting uses First-Fit rather than First-Fit Decreasing. This only matters
  for projects large enough to need splitting at all.
- The "one finding per line" rule applies only when the keyword cascade fired; on a line
  without a keyword, a provider signature and the entropy detector can both report.
- Redaction recognises encoded secrets, but a short word-like password inside a URL,
  before the first separator, is indistinguishable by shape from a host name.
- `codepack-reports` checks cancellation between reports, not inside each report's file
  loop. Accepted deliberately: the pipeline level already bounds the risk.
- Cancelling while a single very large file is being packed into an archive is not
  interruptible until that file finishes.

## What came before

The previous version was Project Exporter Desktop 1.0.1: Python 3.11+, PySide6, roughly
13,400 lines, Windows only, distributed with PyInstaller and Inno Setup. It has been
removed from the working tree and preserved as an archive, which remains the behavioural
reference for exact constants and artifact formats.

It was rewritten for cross-platform reach, for performance without the GIL, for static
typing, to replace flat JSON storage with SQLite, and to strengthen the secret detector.
