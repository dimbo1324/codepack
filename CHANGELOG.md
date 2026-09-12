# Changelog

Releases of codepack. Newest first. Dates are the day the version was tagged.

This file is for people who use codepack. The rule-system changelog for the AI assistants
that build it is a separate file, `.ai/CHANGELOG.md`.

## Unreleased

### Ask a model about a bundle

codepack can now send a finished bundle's AI context to a provider and bring the answer
back, from the command line (`codepack ask`) or from the desktop app's Result page. This
completes the half of the AI integration that has existed in the code, untouchable, since
July.

**It is off until you switch it on.** On a fresh installation nothing here can open a
connection: the refusal happens before your bundle is read and before your key is touched.

Your API key goes in your operating system's credential store — Credential Manager,
Keychain or Secret Service — and never in the settings file, which is a file you can
export and hand to a colleague. `codepack key set` reads the key from stdin rather than
from a flag, because an argument is visible to every other process on the machine and
lands in your shell history; piping from a password manager (`pass show anthropic |
codepack key set`) is the shape it is built for. The desktop app has a masked field for
typing one instead. There is no command that prints a stored key.

**A bundle with critical security findings is refused.** Sending one would make the
scanner decorative. Overriding that is a flag you type (`--override-critical`) or a
checkbox you tick, never a consequence of pressing send. `codepack ask --dry-run` shows
exactly what would be sent — how many files, how large, roughly how many tokens, and what
the scan found — and sends nothing; a bundle nothing has scanned is reported as **not
verified**, which is not the same as clean.

Answers are appended to `AI_ANSWER.md` inside the bundle, so a second question does not
destroy the first answer.

### The window fits your screen, and you can zoom it

The app used to open at a fixed 1100×760 regardless of the display it opened on. On a
1920×1200 panel at 150% scaling — a 1280×752 work area — that window was **taller than
the space available**, and the layout was squeezed from the first pixel. It now measures
the monitor and treats the configured size as a ceiling, centred inside the usable area
rather than under your taskbar.

The interface scale adapts too. On a small or heavily scaled display it starts zoomed out
instead of cramped, recalculated each launch, so plugging in a bigger monitor adapts
rather than keeping a scale that suited the laptop. It only ever scales *down* on its own:
enlarging would override the display scaling you chose in your operating system.

And zoom is finally reachable — `Ctrl` `+`, `Ctrl` `-`, `Ctrl` with the mouse wheel, and a
readout with two buttons in the status bar. `Ctrl` `0` hands the scale back to the monitor.
Anything you set is remembered. The slider in Settings also works properly now: its range
was 75–200% while the application only accepts 70–150%, so the most useful setting on a
small screen could not be reached at all, and 200% displayed a window that was at 150%.

### Still true, and still worth saying

Everything else in codepack remains local. Exactly one crate in the whole project is
allowed to reach the network, only the two front ends may even reach that crate, and the
quality gate fails the build if either rule is broken — so no export can carry a request
underneath itself.

### Not done

No live request to a provider has been made by anyone building this: the network leg is
covered by tests and response parsing, not by a real exchange. A send in progress cannot
be cancelled.

## 2.0.1 — 2026-09-08

A correctness and durability release. Nothing you do with codepack changes; what changes
is that Linux packaging now genuinely works, secrets no longer reach places they were
never meant to, and the project has the logs, the release channel and the checks it needs
to sit unattended without quietly rotting.

### Linux is real now

The `.deb` and `.rpm` carry the CLI as well as the desktop app: `/usr/bin/codepack`, its
man page, and completions for bash, zsh and fish. Every released package is installed and
run in a clean `ubuntu:24.04`, `debian:12` and `fedora:41` container before it ships —
which is how the next item was found.

**The CLI would not start on Debian 12 at all.** Built on the CI runner's own Ubuntu
24.04, it required `GLIBC_2.39`; Debian 12 has 2.36, and glibc only ever promises forward
compatibility. It is now cross-built inside a `debian:12` container — the oldest
distribution supported — so one binary runs on all three.

The database and the extracted-bundle cache moved to `$XDG_DATA_HOME` on Linux, out of the
config directory people sync between machines. Existing installations migrate; nothing is
lost. The log directory honours `$XDG_STATE_HOME`. The window no longer opens blank on
common NVIDIA and Wayland setups — the workaround is documented in the README, and a
failed system tray no longer aborts startup.

### Secrets

A secret in the live progress narration was **not** redacted before reaching the CLI's
stderr and the desktop's own window. It is now, through the same function every other
surface already used. `codepack init --hook` no longer follows a `core.hooksPath` that
points outside the repository. The secret scanner reads a very large file up to its
ceiling instead of pulling the whole thing into memory, and an external formatter that
hangs can no longer hang the run with it.

### An activity log

`codepack` now keeps its own log: what an export did, what it skipped and why, rotated
daily and swept by age and total size, with every line redacted on the way in. A release
build that panics writes a separate `codepack-panics.log` — before this, a crash was
completely silent. `codepack doctor --collect-logs DIR` gathers it for a bug report with
your home directory replaced by `<home>`.

### Getting it

Every tagged version is now published as a [GitHub
Release](https://github.com/dimbo1324/codepack/releases), built by CI rather than on
someone's machine, with a build-provenance attestation on each file — verifiable proof of
which workflow run and which commit produced it, which a checksum alone cannot give.
`setup.exe` at the repository root tracks the latest Windows installer, and `SETUP.txt`
beside it records the version, build time, commit and checksum.

### Still not in this release

Unchanged from 2.0.0: no code signing or notarisation, no macOS bundle, no auto-update,
and no interface to the hosted-model API path. The desktop binary in the Linux packages
still carries the newer glibc requirement the CLI no longer does.

## 2.0.0 — 2026-09-06

The first release of the Rust rewrite. The previous product was Project Exporter Desktop
1.0.1 (Python/PySide6, Windows only); this shares its behaviour and none of its code.

### What it does

- **Export.** A source folder becomes an archive plus about thirty reports — structure,
  stack, dependencies, git, security, tokens, an AI context pack and an HTML dashboard.
- **Safety modes.** `safe` (the default), `balanced` and `full` decide what is copied at
  all. Safe mode excludes `.env` files, key material and similarly named files.
- **Secret scanning.** Provider signatures, entropy and structural parsing, with a
  precision/recall corpus test that the build refuses to fall below.
- **Two front ends over one engine.** A desktop app (Tauri + Svelte) and a headless CLI
  with `--json` on every command. Neither shells out to the other.
- **Differential export**, snapshots and history in SQLite.
- **Sterile copy** — strip comments with tree-sitter and reformat with a `PATH` tool into
  a separate destination.
- **Handoff to a local agent**, and an MCP server so an agent can ask for itself.
- **Pre-commit hook** and a GitHub Action.

### Guarantees held by tests

Analysis is entirely local — no crate in the workspace can reach the network. The source
folder is never written to. Secrets never reach a report, a log, the history, the
database or an error message. Symlinks are never followed; extraction is path-traversal
safe and bounded.

### Platforms

Windows 10 and 11, macOS, and Linux: the quality gate runs on all three. **Only Windows
has an installer** — an NSIS `.exe` with a `SHA256SUMS.txt` beside it.

### Not in this release

- Code signing and notarisation. SmartScreen warns about an unknown publisher.
- macOS and Linux installers.
- Auto-update.
- The API path to a hosted model. The offline handoff works; the HTTP client is preserved
  in a package excluded from the build and has no interface yet.
- Mermaid diagram rendering, and `file:line` links in the review checklist.
