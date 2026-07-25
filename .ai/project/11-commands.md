# Project Commands and Quality Gates

All commands run from the repository root (Windows: PowerShell or Git Bash).

## Main entry point — the xtask runner

```powershell
cargo xtask gate            # full quality gate — the main verification path
cargo xtask gate --quick    # quick gate — the minimum before a push
cargo xtask fmt             # format Rust sources
cargo xtask lint            # clippy with warnings denied
cargo xtask test            # workspace tests
cargo xtask deny            # cargo-deny: advisories, bans, licenses, sources
cargo xtask sync-agents     # regenerate AGENTS.md from the .ai/ modules
cargo xtask sync-agents --check   # verify AGENTS.md is in sync
cargo xtask doctor          # read-only environment diagnostics
cargo xtask golden          # regenerate the legacy golden references (needs Python)
```

`cargo xtask gate` runs formatting, clippy, tests, `cargo deny check`, and the
`AGENTS.md` sync check. Prefer it over ad-hoc command sequences.

`cargo xtask golden` runs the archived legacy implementation and rewrites
`tests/golden/reference/`. It is a developer-machine command: it needs Python 3 on
`PATH`, and CI never runs it — the references are committed, so the Rust suite compares
against files. Run it only when legacy's own output should change, never to make a
failing comparison pass.

`cargo deny check` requires the `cargo-deny` binary (`cargo install cargo-deny`,
not a `rust-toolchain.toml` component — CI installs it via `taiki-e/install-action`).
`cargo xtask doctor` reports whether it is on `PATH`.

## Direct commands when targeting one layer

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
pnpm install --frozen-lockfile
pnpm --filter @codepack/ui typecheck
pnpm --filter @codepack/ui lint
```

Frontend commands require `pnpm install` once. The frontend lives in
`apps/desktop/ui`; the Tauri shell arrives in stage S11.

## Gate policy

- Before merging to `main`, the full gate must be green.
- The quick gate is the minimum for intermediate pushes.
- `sync-agents --check` is part of the gate: a drift between `AGENTS.md` and the `.ai/`
  modules breaks the build on purpose.
- Documentation-only or configuration-only changes still run the gate.
- CI runs a matrix of `windows-latest`, `macos-latest`, and `ubuntu-latest`:
  cross-platform support is verified automatically, not claimed.

## Platform notes

- Windows: long paths and antivirus can interfere with temporary directories; prefer a
  repository-local temp directory in tests.
- macOS: building the Tauri shell requires Xcode Command Line Tools.
- Linux: the Tauri shell requires `webkit2gtk` and related system libraries. Core crates
  and the CLI have no such dependency and build anywhere.

## Toolchain

The Rust toolchain is pinned in `rust-toolchain.toml`; do not bypass it. Node and pnpm
versions are declared in `package.json` under `engines` and `packageManager`.
