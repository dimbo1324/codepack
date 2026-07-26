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

- [+] Orientation ritual done (git, ROADMAP, overview, previous checklist, decisions)
- [+] Baseline recorded: 913 tests, golden 3/3, `cargo xtask gate` green locally
- [+] Checklist committed before any code

## Task 1 — Windows-only toolchain

- [+] `.github/workflows/ci.yml`: matrix reduced to `windows-latest`; the macOS/Linux
      legs and the Linux system-dependency step commented out with a `TODO` pointing at
      the open question, not deleted
- [+] `codepack-core::paths`: the `Os::Mac`/`Os::Linux` layout arms commented with a
      `TODO`; Windows is the only resolved layout
- [+] The two layout tests for macOS/Linux commented alongside the code they cover, so a
      commented branch never looks tested
- [+] `.ai/project/11-commands.md`: platform notes and gate policy match a Windows-only
      reality
- [+] `docs/decisions/open-questions.md`: the narrowing recorded, plus the still-unknown
      Unix gate failure logged as Q21 so dropping those legs does not bury it

## Task 2 — Strict auto-formatting

- [+] `prettier-plugin-svelte` added (Prettier cannot format `.svelte` without it)
- [+] `prettier.config.mjs` and `.prettierignore` at the repository root. Chose `.mjs`
      over `.prettierrc` so the non-obvious choices carry their reasons; `printWidth: 100`
      matches `rustfmt.toml`'s `max_width`, everything else is Prettier's default because
      that is already what the tree looked like
- [+] `pnpm format` (check) kept, `pnpm format:write` added; both moved to the workspace
      root so one config and one ignore file apply regardless of the directory you run in
- [+] `cargo xtask fmt` formats Rust **and** the frontend
- [+] `cargo xtask install-hooks` installs a `pre-commit` hook via `core.hooksPath`
- [+] The hook formats **only staged files** and re-stages them; it degrades to a warning
      when `node_modules` is absent instead of blocking a Rust-only commit
- [+] Whole tree formatted once, in its own commit, so the mechanical diff stays separate
      from the logic diff
- [+] Frontend format/typecheck/lint join `cargo xtask gate`, skipping with a clear
      message when `node_modules` is absent
- [+] `.ai/project/11-commands.md` documents the new commands
- [+] **Unplanned, found on the way:** `.editorconfig` set `indent_size = 4` for `[*]` and
      listed neither `.svelte` nor `.mjs` in its two-space override. Nothing enforced the
      file, so those sources were hand-written with two spaces and silently disagreed with
      it — Prettier's first run reindented 14 files (~1200 lines). Fixed the root cause
      instead of overriding it in Prettier: the churn dropped to 5 files, 19 insertions

## Task 3 — Installable `.exe`

- [+] `bundle.targets` set to NSIS so the artifact is a single `.exe` installer
- [+] `mainBinaryName` set: the binary is `codepack-desktop` while `productName` is
      `codepack`, and the bundler resolves the executable from the product name — it would
      have looked for a `codepack.exe` that does not exist
- [+] NSIS `installMode: currentUser`, so installing needs no admin prompt; installer
      languages English + Russian
- [+] `cargo xtask package` drives the whole build (frontend → Tauri → installer)
- [+] Installer really built: `target/release/bundle/nsis/codepack_2.0.0_x64-setup.exe`,
      4.4 MiB, release build 8m47s
- [+] `.ai/project/11-commands.md` and `ROADMAP.md` stop claiming `tauri build` is S14-only
- [+] **Unplanned, found on the way:** the documented dev command
      `pnpm --filter @codepack/ui exec tauri dev` never worked. The Tauri CLI locates a
      project by finding `tauri.conf.json` in a *subfolder*, and `src-tauri` is a sibling
      of `ui/`. Both commands now run from `apps/desktop`, and the CLI moved to the
      workspace root so it resolves from there

## Task 4 — Single instance

- [+] `tauri-plugin-single-instance` added (new production dependency — named in the
      report with its justification; `cargo deny check` passes with it)
- [+] Registered first among plugins, per the plugin's own requirement
- [+] A second launch focuses the existing window instead of starting a second process:
      restores it if minimised, shows it if hidden, raises it if behind other windows.
      The tray's "Show" item now shares that one helper and gained the `unminimize()` it
      was missing
- [+] Only one tray icon can exist, because only one process can. This also closes a
      second-order problem nobody had reported: two instances opened two connections to
      the same SQLite history database

## Verification

- [+] `cargo fmt --all --check`
- [+] `cargo clippy --workspace --all-targets -- -D warnings`
- [+] `cargo test --workspace` — 911. Exactly the 913 baseline minus the two commented-out
      macOS/Linux layout tests; nothing else moved
- [+] `cargo test -p codepack-engine --test golden` — 3/3
- [+] `pnpm --filter @codepack/ui typecheck` (102 files, 0/0), `lint`, `format`, `build`
      (bundle byte-identical at 86.73 kB, proving the reformat was cosmetic)
- [+] `cargo deny check` — clean, including the new plugin's licence
- [+] `cargo xtask sync-agents --check`
- [+] `cargo xtask gate` end to end, exit 0, with the frontend steps really running
- [+] Pre-commit hook exercised end to end on throwaway commits: a misformatted `.ts` and
      `.rs` came out formatted, and a partially staged file was skipped with its unstaged
      half correctly left out of the commit. Probe commits then removed
- [+] Single instance verified on the built release binary: three launches, one process
- [-] Installer built (4.4 MiB) but **not installed and launched from the installed copy**.
      Verified the binary it wraps instead. Installing would write to this machine's
      Program Files and Start menu, which is the owner's call, not mine — see the report
- [ ] Independent review of the diff
- [ ] CI green on `windows-latest`

## Completion

- [+] `ROADMAP.md` reflects the Windows-only scope and the pulled-forward installer
- [+] `docs/architecture/overview.md` updated
- [+] `docs/decisions/open-questions.md` carries every decision and open question above
- [+] Checklist filled `+`/`-` honestly, final report written in Russian

## Rule debt

`AGENTS.md` now assembles to 29.9 KiB against a 30 KiB budget. This task nearly broke it
and had to tighten its own additions to fit, which means the **next** module addition will
break it. That is a real constraint on the next agent and needs either a raised budget or a
module demoted to `tier: extended` — an owner decision, not a silent one.

## Next task

Either S13 (AI integration) or the return of cross-platform support, whichever the owner
picks. The open questions recorded here are what makes the second one possible without
re-deriving it.
