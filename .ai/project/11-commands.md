# Project Commands and Quality Gates

All commands run from the repository root (Windows: PowerShell or Git Bash).

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

`cargo xtask gate` runs formatting, clippy, tests, `cargo deny check`, the frontend
`format`/`typecheck`/`lint` checks, and the `AGENTS.md` sync check. Prefer it over ad-hoc
command sequences.

## Formatting

Two formatters, one command. `rustfmt` owns `.rs` (config in `rustfmt.toml`); Prettier
owns the frontend and the repository's own `.md`/`.json`/`.yml` (config in
`.prettierrc`, exclusions in `.prettierignore`).

```powershell
cargo xtask fmt             # write mode, both toolchains
cargo xtask install-hooks   # once per clone
```

`install-hooks` points `core.hooksPath` at the tracked `.githooks/` directory, so the hook
is versioned with the repository instead of living in an untracked `.git/hooks`. The
`pre-commit` hook formats **only the files that are staged** and re-stages them, which
keeps a commit from quietly absorbing unrelated reformatting of files you left dirty on
purpose. If `node_modules` is missing it formats the Rust half and prints a notice rather
than blocking the commit.

To commit without the hook once: `git commit --no-verify`. The gate still catches
formatting afterwards, so this delays the check rather than skipping it.

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
pnpm --filter @codepack/ui build
```

Frontend commands require `pnpm install` once. The frontend lives in
`apps/desktop/ui`; the Tauri shell is `apps/desktop/src-tauri` (crate
`codepack-desktop`), a normal member of the cargo workspace — `cargo xtask gate` builds
and tests it like any other crate.

Running the app in development needs both halves, which the Tauri CLI starts together:

```powershell
pnpm --filter @codepack/ui exec tauri dev
```

Producing the Windows installer:

```powershell
cargo xtask package
```

It builds the frontend, then the Tauri release bundle, and leaves an NSIS `.exe`
installer under `target/release/bundle/nsis/`. Code signing, notarisation,
`SHA256SUMS.txt`, and auto-update remain stage S14 — only the installer itself was pulled
forward, by owner decision 2026-07-26.

## Gate policy

- Before merging to `main`, the full gate must be green.
- The quick gate is the minimum for intermediate pushes.
- `sync-agents --check` is part of the gate: a drift between `AGENTS.md` and the `.ai/`
  modules breaks the build on purpose.
- Frontend `format`/`typecheck`/`lint` are part of the gate. They are skipped with a
  printed notice when `apps/desktop/ui/node_modules` is absent, so a Rust-only checkout
  still gates — but CI installs dependencies, so there they always run.
- Documentation-only or configuration-only changes still run the gate.
- CI runs `windows-latest` only. Owner decision 2026-07-26 narrowed the build scope to
  Windows 10/11; the `macos-latest` and `ubuntu-latest` legs are commented out in
  `.github/workflows/ci.yml` rather than deleted.

## Platform notes

The supported target is **Windows 10 and Windows 11**. macOS and Linux are out of scope
for now (BLUEPRINT §B.4 still declares them a product goal; see
`docs/decisions/open-questions.md` for the decision and Q21 for what has to be
re-diagnosed when they return).

- Windows: long paths and antivirus can interfere with temporary directories; prefer a
  repository-local temp directory in tests.
- Cross-platform code that had to be switched off is **commented with a
  `TODO(cross-platform)` marker**, never deleted — `codepack-core::paths` is the one place
  in the domain crates where this applies. Grep that marker to find everything that must
  come back together.
- Test helpers under `#[cfg(unix)]` are left in place: they do not compile on Windows, so
  they cost nothing here, and they carry the invariant I7 symlink coverage.

## Toolchain

The Rust toolchain is pinned in `rust-toolchain.toml`; do not bypass it. Node and pnpm
versions are declared in `package.json` under `engines` and `packageManager`.
