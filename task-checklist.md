# Task Checklist

**Task:** Narrow the whole toolchain to Windows 10/11, add strict auto-formatting on
commit, produce an installable `.exe`, and make the app single-instance.
**Date:** 2026-07-26
**Branch:** feat/windows-only-toolchain-and-installer

Four owner-requested tasks in one branch, because they share one theme: take what S0–S12
already built and finish it into something installable and maintainable on one platform,
rather than half-working on three. Stages S13 and S14 stay untouched as features.

Note on language: this file is English per `.ai/project/10-project-map.md`; the S12
checklist was written in Russian, which violated that rule.

## Owner decisions that shape this task

1. **Windows-only for now.** macOS and Linux leave CI and the code paths. Cross-platform
   code is commented with a `TODO` naming what returns and when — not deleted, because
   BLUEPRINT §B.4 still declares cross-platform a product goal. BLUEPRINT is **not**
   rewritten: the product intent has not changed, only the current build scope, so the
   narrowing is recorded in `docs/decisions/open-questions.md` instead.
2. **The installer is S14 scope, pulled forward on purpose.** `ROADMAP.md` §1 puts
   packaging in S14 and stage order is binding, so this is an explicit owner decision
   and gets recorded as one. Only the Windows installer comes forward; signing,
   notarisation, `SHA256SUMS.txt`, and auto-update stay in S14.
3. **Test helpers under `#[cfg(unix)]` stay.** They are dormant on Windows (the branch
   does not compile there) and they exist to prove invariant I7 (symlinks are never
   followed). Commenting them out would cost real safety coverage and buy nothing, so
   they are left alone and called out in the final report.

## Preparation

- [ ] Orientation ritual done (git, ROADMAP, overview, previous checklist, decisions)
- [ ] Baseline recorded: 913 tests, golden 3/3, `cargo xtask gate` green locally
- [ ] Checklist committed before any code

## Task 1 — Windows-only toolchain

- [ ] `.github/workflows/ci.yml`: matrix reduced to `windows-latest`; the macOS/Linux
      legs and the Linux system-dependency step commented out with a `TODO` pointing at
      the open question, not deleted
- [ ] `codepack-core::paths`: the `Os::Mac`/`Os::Linux` layout arms commented with a
      `TODO`; Windows is the only resolved layout
- [ ] The two layout tests for macOS/Linux commented alongside the code they cover, so a
      commented branch never looks tested
- [ ] `.ai/project/11-commands.md`: platform notes and gate policy match a Windows-only
      reality
- [ ] `docs/decisions/open-questions.md`: the narrowing recorded, plus the still-unknown
      Unix gate failure logged so dropping those legs does not bury it

## Task 2 — Strict auto-formatting

- [ ] `prettier-plugin-svelte` added (Prettier cannot format `.svelte` without it)
- [ ] `.prettierrc` and `.prettierignore` at the repository root; settings chosen to
      match the code already in the tree, not to churn it
- [ ] `pnpm format` (check) kept, `pnpm format:write` added
- [ ] `cargo xtask fmt` formats Rust **and** the frontend
- [ ] `cargo xtask install-hooks` installs a `pre-commit` hook via `core.hooksPath`
- [ ] The hook formats **only staged files** and re-stages them; it degrades to a warning
      when `node_modules` is absent instead of blocking a Rust-only commit
- [ ] Whole tree formatted once, in its own commit, so the mechanical diff stays separate
      from the logic diff
- [ ] Frontend format/typecheck/lint join `cargo xtask gate`, skipping with a clear
      message when `node_modules` is absent
- [ ] `.ai/project/11-commands.md` documents the new commands

## Task 3 — Installable `.exe`

- [ ] `bundle.targets` set to NSIS so the artifact is a single `.exe` installer
- [ ] `mainBinaryName` set: the binary is `codepack-desktop` while `productName` is
      `codepack`, and the bundler resolves the executable from the product name
- [ ] NSIS `installMode` chosen so a normal user can install without an admin prompt
- [ ] `cargo xtask package` drives the whole build (frontend → Tauri → installer)
- [ ] The installer is actually built and its path/size reported — a build command that
      was never run is not a deliverable
- [ ] `.ai/project/11-commands.md` and `ROADMAP.md` stop claiming `tauri build` is S14-only

## Task 4 — Single instance

- [ ] `tauri-plugin-single-instance` added (new production dependency — named in the
      report with its justification)
- [ ] Registered first among plugins, per the plugin's own requirement
- [ ] A second launch focuses the existing window instead of starting a second process:
      restores it if minimised, shows it if hidden, raises it if behind other windows
- [ ] Only one tray icon can exist, because only one process can
- [ ] Behaviour verified against the built installer, not only in `tauri dev`

## Verification

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace` — no regression against the 913 baseline
- [ ] `cargo test -p codepack-engine --test golden` — 3/3
- [ ] `pnpm --filter @codepack/ui typecheck`, `lint`, `format`, `build`
- [ ] `cargo deny check`
- [ ] `cargo xtask sync-agents --check`
- [ ] `cargo xtask gate` end to end
- [ ] Pre-commit hook exercised on a real commit with a deliberately misformatted file
- [ ] Installer built, installed, launched, and double-launch tested for one instance
- [ ] Independent review of the diff
- [ ] CI green on `windows-latest`

## Completion

- [ ] `ROADMAP.md` reflects the Windows-only scope and the pulled-forward installer
- [ ] `docs/architecture/overview.md` updated
- [ ] `docs/decisions/open-questions.md` carries every decision and open question above
- [ ] Checklist filled `+`/`-` honestly, final report written in Russian

## Next task

Either S13 (AI integration) or the return of cross-platform support, whichever the owner
picks. The open questions recorded here are what makes the second one possible without
re-deriving it.
