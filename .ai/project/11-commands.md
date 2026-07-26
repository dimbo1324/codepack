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

`rustfmt` owns `.rs` (`rustfmt.toml`); Prettier owns the frontend and config files
(`prettier.config.mjs`, exclusions in `.prettierignore` — which protects `tests/golden/`,
test fixtures, and the generated `AGENTS.md`). `cargo xtask fmt` runs both.

Run `cargo xtask install-hooks` once per clone. It points `core.hooksPath` at the tracked
`.githooks/`, so the hook is versioned rather than living in an untracked `.git/hooks`.
The `pre-commit` hook formats **only staged files** and re-stages them; it skips a
partially staged file rather than sweeping its unstaged half into the commit, and skips
Prettier with a notice when `node_modules` is absent. Bypass once with
`git commit --no-verify` — the gate still checks formatting later.

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
pnpm format                        # Prettier, check only
pnpm format:write                  # Prettier, rewrite in place
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
pnpm desktop:dev
```

**The working directory matters.** The Tauri CLI locates a project by finding
`tauri.conf.json` in a *subfolder* of the current directory. Here the shell
(`apps/desktop/src-tauri`) sits beside the frontend (`apps/desktop/ui`), so from `ui` the
config is a sibling and the CLI aborts — which is why the previously documented
`pnpm --filter @codepack/ui exec tauri dev` never worked. Both scripts run from
`apps/desktop`, and the CLI is a workspace-root dev dependency so it resolves there.

Producing the Windows installer:

```powershell
cargo xtask package
```

Leaves an NSIS `.exe` under `target/release/bundle/nsis/`. Signing, notarisation,
`SHA256SUMS.txt`, and auto-update stay in S14 — only the installer was pulled forward
(owner decision 2026-07-26).

## Gate policy

- Before merging to `main`, the full gate must be green.
- The quick gate is the minimum for intermediate pushes.
- `sync-agents --check` is part of the gate: a drift between `AGENTS.md` and the `.ai/`
  modules breaks the build on purpose.
- Frontend `format`/`typecheck`/`lint` are part of the gate. Without
  `apps/desktop/ui/node_modules` they skip with a notice, so a Rust-only checkout still
  gates — but when `CI` is set they **fail** instead, because a silent skip there would let
  unformatted frontend code pass.
- Documentation-only or configuration-only changes still run the gate.
- CI runs `windows-latest` only (owner decision 2026-07-26); the other two legs are
  commented out in `.github/workflows/ci.yml`, not deleted.

## Platform notes

The supported target is **Windows 10 and Windows 11**. macOS and Linux are out of scope for
now — BLUEPRINT §B.4 still calls them a product goal; see `open-questions.md` for the
decision and Q21 for what must be re-diagnosed first.

- Windows: long paths and antivirus can interfere with temporary directories; prefer a
  repository-local temp directory in tests.
- Switched-off cross-platform code is **commented with `TODO(cross-platform)`**, never
  deleted. Grep that marker to find everything that must return together;
  `codepack-core::paths` is the only domain crate affected.
- Test helpers under `#[cfg(unix)]` stay: they do not compile on Windows, so they cost
  nothing, and they carry the invariant I7 symlink coverage.

## Toolchain

The Rust toolchain is pinned in `rust-toolchain.toml`; do not bypass it. Node and pnpm
versions are declared in `package.json` under `engines` and `packageManager`.
