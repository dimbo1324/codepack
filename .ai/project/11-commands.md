# Project Commands and Quality Gates

All commands run from the repository root (Windows: PowerShell or Git Bash).

## The script orchestrator — start here

```powershell
python dev_tools_scripts_runner.py          # interactive menu
python dev_tools_scripts_runner.py list     # machine-readable catalog — use this, not the menu
python dev_tools_scripts_runner.py <name>   # run one directly; `help` prints manuals
```

Cross-platform door to the routine jobs: `quality-gate` (default), `format-code`,
`dev-run`, `build-installer`, `doctor`, `install-hooks`, `clean-project`, `selftest`.
With no arguments and no terminal it runs `quality-gate` rather than blocking on input,
which is what makes it usable by an agent. The scripts wrap the `cargo xtask` commands
below instead of reimplementing them, so both doors reach the same code.

**`clean-project` deletes files.** Dry run by default; never touches `.env`, signing
material, local databases, or a nested git repository. Read its
`config/clean.json` first.

**Standing duty — keep the scripts accurate and portable.** They are infrastructure
everyone relies on, so a task that changes how the project is built, checked, formatted,
run, or cleaned updates the matching script *in that same task*; a script describing a
workflow that no longer exists is worse than none. New routine work gets a new script:
`scripts/<name>/__main__.py` plus one entry in `scripts/runner/config/scripts.json` —
adding a script changes no Python in `scripts/runner/`. Settings live in each script's
own `config/*.json`; scripts never import each other, only `scripts/_toolkit`. Resolve
tools through `_toolkit/processes.py` (a bare `"pnpm"` does not resolve on Windows), and
let genuinely Windows-only work refuse with a reason instead of failing part-way. Run
`selftest` after touching anything under `scripts/`.

## Main entry point — the xtask runner

```powershell
cargo xtask gate            # full quality gate — the main verification path
cargo xtask gate --quick    # quick gate — the minimum before a push
cargo xtask fmt             # format Rust *and* frontend sources in place
cargo xtask lint            # clippy with warnings denied
cargo xtask test            # workspace tests
cargo xtask deny            # cargo-deny: advisories, bans, licenses, sources
cargo xtask sync-agents     # regenerate AGENTS.md from the .ai/ modules
cargo xtask sync-agents --check   # verify AGENTS.md is in sync
cargo xtask install-hooks   # install the formatting pre-commit hook
cargo xtask package         # build the Windows NSIS installer
cargo xtask doctor          # read-only environment diagnostics
cargo xtask golden          # regenerate the legacy golden references (needs Python)
```

Prefer `gate` over ad-hoc command sequences.

`cargo deny check` needs the `cargo-deny` binary installed separately (`cargo install
cargo-deny`, not a toolchain component; CI uses `taiki-e/install-action`).

`cargo xtask golden` re-runs the archived legacy implementation to rewrite
`tests/golden/reference/`. Developer-machine only: it needs Python 3, and CI never runs
it because the references are committed. Run it when legacy's own output *should* change
— never to make a failing comparison pass.

## Formatting

`rustfmt` owns `.rs` (`rustfmt.toml`); Prettier owns the frontend and config files
(`prettier.config.mjs`; `.prettierignore` protects `tests/golden/`, test fixtures, and
the generated `AGENTS.md`). `cargo xtask fmt` runs both.

`install-hooks` once per clone points `core.hooksPath` at the tracked `.githooks/`, so
the hook is versioned instead of living in an untracked `.git/hooks`. The `pre-commit`
hook formats **only staged files** and re-stages them; it skips a partially staged file
rather than sweeping its unstaged half into the commit, and skips Prettier with a notice
when `node_modules` is absent. `git commit --no-verify` bypasses it once — the gate still
checks formatting later.

## Where the rest lives

Per-layer commands, the Tauri working-directory trap, `cargo deny`/`golden` notes, and the
platform notes are in `15-command-reference.md` — lookup material, kept separate so this
module stays the part that applies to every task.

## Gate policy

- The full gate must be green before merging to `main`; the quick gate is the minimum for
  intermediate pushes. Documentation- and configuration-only changes still run it.
- `sync-agents --check` is part of the gate: drift between `AGENTS.md` and `.ai/` breaks
  the build on purpose.
- Frontend `format`/`typecheck`/`lint` are part of it too. Without
  `apps/desktop/ui/node_modules` they skip with a notice so a Rust-only checkout still
  gates — but with `CI` set they **fail** instead, since a silent skip there would let
  unformatted frontend code through.
- CI runs `windows-latest` only (owner decision 2026-07-26); the other legs are commented
  out in `.github/workflows/ci.yml`, not deleted.
