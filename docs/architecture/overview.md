# Architecture: what exists today

> This document describes what is **actually in the code**, not what is planned. It is
> updated whenever the shape of the system changes: a new crate, a new layer, a new
> operational job.
>
> Rewritten in English on 2026-07-30, when the documentation was split into internal and
> external sets. The per-date engineering history that used to live here is in the git
> log and in the internal plan; this file answers "what is built and how does it fit
> together".

**Last revised:** 2026-09-07 · **Version:** 2.0.0
**Target platforms:** Windows 10/11, macOS and Linux. The 2026-07-26 narrowing to
Windows was reversed on 2026-09-06; `codepack-core::paths` carries all three layouts
again and CI runs the gate on all three runners. Packaging followed on the same day:
`cargo xtask package` produces an NSIS `.exe` on Windows and `.deb`/`.rpm`/`.AppImage`
on Linux (`tauri.conf.json`'s `bundle.linux`), each with dependencies declared per
distribution and a `SHA256SUMS.txt` beside it. There is still no macOS bundle
(stage S14), and none of the produced packages are signed yet.

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
   ↑
codepack mcp             the MCP server, inside the CLI binary
```

Dependencies point strictly downward and there are no cycles. The two front ends sit
**side by side** over the engine — the desktop app calls `codepack-engine` directly, it
does not shell out to the CLI. The MCP server is a third *surface* rather than a third
front end: it lives inside `codepack-cli` and calls that binary's own command builders,
because what a preset means and why `scan` forces safe mode to `full` are the CLI's
decisions, not the engine's, and restating them elsewhere is how two answers to one
question appear. No `codepack-*` crate knows about Tauri or the frontend
(invariant I8), so the whole core builds and tests headless.

## The crates

| Crate | What it does |
|---|---|
| `codepack-core` | Domain types and `Config` (27 fields plus `schema_version`), normalization, migration from the legacy settings file, the six AI presets, `AppPaths`, `CancellationToken`, progress and log events. Also the single home for things that were once duplicated: text/binary classification (`classify`), the civil-date algorithm (`time`), and the `.codepack-allow` format and fingerprint recipe (`allowlist`). |
| `codepack-scanner` | Walks the tree, applies ignore rules (base, per-stack, `.exportignore`, user rules), detects the stack, and builds the export plan. Symlinks are never followed (invariant I7). |
| `codepack-security` | Safe-export modes, secret redaction (plain, or with a stable per-secret label — `Redactor`), and the detector: provider signatures, entropy, a keyword cascade and named risky-code shapes. Carries an accuracy corpus test whose precision/recall thresholds may never be lowered (invariant I9). Emits SARIF 2.1.0. |
| `codepack-diff` | Differential export and snapshots through `git2`. Never requires a `git` binary. |
| `codepack-storage` | SQLite: seven tables plus `schema_version`, numbered migrations, run history, snapshots, findings, and per-project retention. Has no runtime dependency on any other `codepack-*` crate. `enable_wal` deliberately never fails when WAL cannot be switched on after contention (a rollback-mode connection still works, just serialises readers behind writers) — `journal_mode()` reads the mode back, and `codepack doctor` surfaces it, so that degradation is observable instead of silent (audit 2026-09-07, P-3/Q-8). |
| `codepack-tokens` | Byte formatting (preserved verbatim from the previous version — invariant I4), token estimation, the budget selection, and `ModelContextLimits`, the model→context-window table that `--budget <model>` resolves through. |
| `codepack-reports` | Around thirty insight reports, `PROJECT_PROFILE.json`, the AI context and prompt folders, the HTML dashboard, and the human-oriented reports (project overview, onboarding guide, review checklist). |
| `codepack-archive` | Archive building and restore. Two entry points: the export pipeline's planned, splittable, reported output, and `pack_files`, which packs a caller-named list of files into one archive. Both honour `ArchiveFormat` — ZIP by default, 7z on request, RAR reserved and refused. Extraction is path-traversal safe (invariant I7). |
| `codepack-sanitize` | The "sterile copy": comments stripped with real tree-sitter parsers (never regex) and code reformatted by whichever formatter is found on `PATH`. Reuses the scanner's file selection, the security crate's safety filter and its redaction — never a second, less guarded path out of the project. Optionally packs the result into one archive. |
| `codepack-engine` | The orchestrator: plan → copy → structure → git → text dump → analytics → manifest → archive. Cancellation is checked inside each step's loops, not only between steps. The only place `codepack_security::scan_project` is called in the pipeline. |
| `codepack-ai` | Stage S13's offline half, and the only part either front end uses: a prompt file and a command for a coding agent already on the machine. No transport, no credential store, no features. |
| `codepack-ai-api` | Stage S13's API path — `ask`, the key store, the Anthropic client. In the repository, **excluded from the workspace**, so `ureq` and `keyring` reach no binary, no `Cargo.lock` and no `cargo deny` graph. Built and linted by `cargo xtask ai-api`, which the gate does not run. Still unreachable by a user; see "Known debt". |
| `xtask` | The task runner and quality gate. |

## The two front ends

**`codepack-cli`** — the `codepack` binary. Twelve commands: `export`, `preview`, `scan`,
`history`, `doctor`, `sanitize`, `completions`, `verify`, `explain`, `handoff` (points a
local coding agent at a bundle), `init --hook` (installs the pre-commit hook into the
user's own project), `settings export|import` (moves one configuration between machines,
so a team's exports come out comparable — Q42). `scan` reads three different file sets — the working tree, the git
index (`--staged`), or every distinct version in the history (`--history`) — writes SARIF
with `--sarif`, and gates on `--fail-on <severity>`, defaulting to `critical` so the
published exit-code contract is unchanged. Its published
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

**`codepack mcp`** — the Model Context Protocol over stdin/stdout (JSON-RPC 2.0,
newline-delimited), so a coding agent on the same machine can ask `preview`, `scan`,
`explain` and `export` itself. stdio only: no dependency was added, no manifest gained a
network client, and the gate's network-isolation step is unaffected. stdout carries
protocol and nothing else — the rule `--json` already lived by, applied to a stream that
is parsed continuously. A tool that fails answers with `isError` inside a successful
result, so the model can read the reason and correct itself; JSON-RPC errors are reserved
for protocol faults.

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
| Quality gate (`cargo xtask gate`) | Eleven sections: format, clippy with warnings denied, tests, `cargo deny`, ignored-advisory review, frontend format/typecheck/lint, the `scripts/` suite, agent-rule sync, report redaction, network isolation, and the installer artifact (audit 2026-09-07, D-1). Runs every section rather than stopping at the first failure, reporting all of them together (audit 2026-09-07, G-2); `xtask::gate_report` writes a timestamped summary, per-command logs, and a JUnit XML for `tests` under `target/gate-logs/<timestamp>/`, refreshes a `target/gate-logs/latest/` copy, keeps the last 20 runs, and — under CI — appends the summary to `$GITHUB_STEP_SUMMARY`. |
| Report redaction | A gate step, like network isolation: raw project file content is reachable only through `text::read_text_unredacted`, and every report that calls it must be declared with a reason in `crates/xtask/src/report_redaction.rs`. The rule used to be "remember to call `redact_line`", and it had already been broken. |
| Machine paths in artifacts | `Config::disclose_absolute_paths`, off by default since 2026-09-06 (Q40). With it off, `source_root` and `copied_root` carry the project's name rather than a path, in all nine places that write them — `PROJECT_PROFILE.json`, `manifest.json`, `28_export_plan.json`, `00_project_profile.json` and five reports. `ReportContext::disclosed_source_root` is the one accessor — `05_git_deep.txt` needed it twice, and the second call site was found only when both Unix runners failed on it, since libgit2 renders even a Windows path with forward slashes and the bundle-wide test searched two spellings; the export plan is set by the engine, since `codepack-scanner` has no business knowing a disclosure policy. No `schema_version` moved: the key and type are unchanged, and an absolute path from another machine was never resolvable by a consumer. |
| Network isolation | A gate step, not a convention: it reads every workspace manifest and fails if any crate declares an HTTP client, or depends on the excluded `codepack-ai-api`. Since 2026-09-06 there is no permitted exception inside the workspace at all. |
| GitHub Action (`action.yml`) | A composite action running `scan` on a runner and emitting SARIF. Builds from source: there are no signed release binaries yet. |
| Dev scripts (`dev_tools_scripts_runner.py`, `scripts/`) | The cross-platform door to routine jobs — quality gate, formatting, dev run, installer, doctor, hooks, clean, selftest. |
| CI (`.github/workflows/ci.yml`) | The `gate` job only, three independent legs — `ubuntu-latest`, `macos-latest`, `windows-latest` — since 2026-09-06. A failing gate emits workflow annotations naming the section and every failing test, because a step's log needs admin rights on the repository and an annotation does not. `permissions: contents: read` at workflow level (audit 2026-09-07, C-2); every third-party action is pinned to a commit SHA with its version as a comment, not a moving tag, with Dependabot (`.github/dependabot.yml`) keeping the pins current, and every `actions/checkout` sets `persist-credentials: false` since no job here pushes (audit 2026-09-07, S-4). Linux packaging validation (`package-linux`, `install-and-run-linux`) moved to its own path-filtered `package-linux.yml` (audit 2026-09-07, C-6): a full release build of the desktop app was 6:28 of a 7:14 run, on every push to any branch, whether or not anything packaging-related had changed. |
| `ai-api-weekly.yml` | Builds, lints and tests the workspace-excluded `codepack-ai-api` (Q41) on a Monday schedule, since `cargo xtask gate` never touches it (audit 2026-09-07, S-12) — insurance against six months of silent rot, at no cost to a push. |
| `perf-smoke-weekly.yml` | Runs `codepack-engine`'s `#[ignore]`-gated `perf_smoke` test (5,000 vs. 50,000 files, checking scaling rather than one absolute number) on a Monday schedule and a fixed runner, since nothing else ever ran it (audit 2026-09-07, T-3/C-3) — before this, a performance regression's first signal was a user complaint, not a red build. `timed_export` now also breaks the total down per pipeline step, by draining the same `StepStarted`/`StepFinished` events both shells already consume rather than adding new instrumentation inside the engine. |
| Packaging | `cargo xtask package` produces an NSIS installer on Windows and `.deb`/`.rpm`/`.AppImage` on Linux, each with a `SHA256SUMS.txt` beside it; a Linux CI job installs the built `.deb` and reads its declared dependencies back out. The `.deb`/`.rpm` also carry `codepack-cli` (`xtask::packaging_assets`, audit 2026-09-07 L-1/L-10): `/usr/bin/codepack`, its man page, and bash/zsh/fish completions, all built and generated by the packaging step itself before `tauri build` runs, since Tauri's `deb.files`/`rpm.files` need the source files already on disk. The AppImage and the NSIS installer do not carry the CLI. Signing, notarisation, auto-update, and a macOS bundle are not done. `.github/workflows/release.yml` (audit 2026-09-07, C-7/S-6, Q48) runs this same command on a `v*` tag push, publishes every artifact to a GitHub Release, and attests each one with `actions/attest-build-provenance` — proof the file was built by this workflow from this commit, which a checksum alone cannot give. GPG-signed Linux packages and a paid Windows certificate remain open (Q48). |
| Activity log (`codepack-engine::LogSink`) | Audit 2026-09-07, G-1: a daily-rotating log file, one more consumer on the existing progress channel rather than a `tracing` dependency, since the pipeline already threads `run=`/`step=` context by hand. `LogLine` is a newtype constructible only through `LogLine::of`, which redacts via `codepack_security::redact_secrets` — the same function the `log`/`log_info` closure in `run_export` now applies *before* sending (S-8: narration used to reach the progress channel, and from there CLI stderr and the desktop webview, unredacted). A chained `panic::set_hook` writes a one-line, redacted entry to a separate non-rotating `codepack-panics.log`, since `strip = "symbols"` plus `windows_subsystem = "windows"` otherwise make a release panic silent. `codepack doctor --collect-logs DIR` copies the log directory out for a bug report, replacing the local home directory with `<home>` first. Both shells wire it in at their earliest startup point; `Config::log_verbose`/`log_max_file_mb`/`log_retention_days`/`log_total_cap_mb` control it, with a desktop settings toggle for verbosity. |
| Extracted-bundle cache (desktop) | `AppPaths::data_dir().join("extracted")` (audit 2026-09-07, S-9) — moved out of `settings_dir()`, which on Linux is `$XDG_CONFIG_HOME` and was accumulating full copies of exported projects, hundreds of megabytes each, in the same directory people sync between machines or commit as dotfiles (the same reasoning that moved the history database out of it, `layout()`'s own doc comment). On Windows and macOS `data_dir()` and `settings_dir()` are the same path, so this only changes anything on Linux. A reproducible cache, not user data: swept once at application startup (`commands::export::migrate_and_sweep_extraction_cache`) — the pre-fix location is discarded outright, and the current one drops any entry unused for 14 days, then the least-recently-used entries until the cache is under 1024 MB, mirroring `LogSink::sweep`'s own two-stage shape without sharing its code (a name-filtered flat-file sweep and a recursive-directory sweep are different enough problems that forcing one abstraction over both would cost more than it saves). |

## Known debt

- **Settings sharing has no per-project scope.** `codepack settings export|import` moves
  the *global* configuration between machines, which is what a team wants for defaults;
  a project that needs different settings still uses `.codepack.toml`. Whether the two
  should be reconcilable — a project file that can be generated from a shared global one
  — has not been asked.
- **`codepack-ai-api` is unreachable by a user, and no longer built by the gate.** Roughly
  eight hundred lines — `keys`, `plan`, `provider`, the Anthropic client and `ask` — have
  no command and no screen behind them. Moving the crate out of the workspace (Q41,
  2026-09-06) removed its dependencies from every platform's build, and the cost is that
  `cargo xtask gate` no longer compiles it: it is preserved, not maintained.
  `cargo xtask ai-api` formats, lints and tests it on demand, and finishing S13 means
  giving this path a command and a screen.
- **Settings import and export are implemented and unwired.**
  `codepack_core::config::{import_settings, export_settings}` are public, tested, and
  called by nothing: no CLI command and no screen offers either. Q42.
- Artifact localization is still a pilot on a single report; the rest of the catalogue
  is English only.
- Redaction labels reach `03_text_dump.txt` and the git reports — the two surfaces an
  assistant reads — but not the ~30 insight reports or scan findings. Widening them into
  the scan artifacts would move `06_security_scan.json` and SARIF, which is an I5 change
  and therefore a separate decision.
- The MCP server handles one request at a time and does not implement cancellation: a
  tool call blocks the loop until it finishes, so a large export cannot be interrupted
  from the client.
- A history scan stops at 500 commits and 8 MB per file version unless told otherwise.
  Both limits are reported rather than silent, but a default run is not a full audit.
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
- The desktop shell opens a fresh SQLite connection (full pragma re-application and
  migration no-op check) on every Tauri command that touches history, rather than
  holding one shared, mutex-guarded connection in `AppState` — audit 2026-09-07, P-3's
  second half. Recorded rather than rushed: sharing one connection across a
  long-running export's own writes and a concurrent `history` read needs careful
  lock-scoping to avoid serialising one behind the other, which is a properly tested
  change, not one to make near the end of an already large remediation pass. The first
  half of P-3 (making a WAL fallback observable) is done; the third — a two-*process*
  WAL-contention regression test, mirroring the real "app and a pre-commit hook running
  at once" scenario `wal_concurrency.rs`'s existing thread-based test cannot reproduce —
  is deferred alongside it for the same reason.

## What came before

The previous version was Project Exporter Desktop 1.0.1: Python 3.11+, PySide6, roughly
13,400 lines, Windows only, distributed with PyInstaller and Inno Setup. It has been
removed from the working tree and preserved as an archive, which remains the behavioural
reference for exact constants and artifact formats.

It was rewritten for cross-platform reach, for performance without the GIL, for static
typing, to replace flat JSON storage with SQLite, and to strengthen the secret detector.
