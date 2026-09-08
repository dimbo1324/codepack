# Task Checklist

**Task:** Final polish before the project is handed over for use and left alone for about
six months: remove unused code, collapse remaining duplication into shared entities, make
the unsafe-looking places predictable, bring every document up to date, clear every
remote branch but `main`, cut release 2.0.1, refresh the build, and re-run everything.

**Date:** 2026-09-08
**Branch:** `chore/2.0.1-final-polish`

Owner instruction, 2026-09-08: plan first, then work through it, and report once done.

## The shape of this task

This is not a feature pass. Everything here either **removes** something (dead code,
duplication, a stale sentence, a branch) or **makes an existing thing more predictable**
(a recompiled regex, an unhandled failure mode). The one addition is the release itself.

The bar for touching code: it must be provable by test or by the gate. Nothing is
"cleaned up" on taste alone — this project is about to sit untouched for months, and a
refactor nobody will re-verify is a liability, not polish.

## Step 0 — preparation

- [ ] Orientation ritual: git state, ROADMAP, overview, previous checklist, open questions
- [ ] Branch `chore/2.0.1-final-polish` off green `main`
- [ ] This checklist committed **before** the work starts

## Step 1 — unused and dead code

- [ ] Compiler-proven dead code: `cargo clippy --workspace --all-targets` with dead-code
      lints, on every target (a `#[cfg]`-gated item can be dead on one platform only)
- [ ] Unused dependencies: every crate's manifest against what it actually imports
- [ ] `#[allow(dead_code)]` sites justified or removed
- [ ] `clean-project` knows about the 770 MB `crates/codepack-ai-api/target/` — the
      excluded crate builds into its own target dir, which nothing currently sweeps

## Step 2 — duplication collapsed into shared entities

- [ ] **Regexes recompiled per call.** 25 `Regex::new` sites in `codepack-reports`;
      exactly one uses `LazyLock`. The rest are `fn x_pattern() -> Regex` called *inside*
      the per-file loop, so every regex is rebuilt once per source file scanned.
      One shape for all of them: `static X: LazyLock<Regex>`.
- [ ] Any other duplicated logic found while doing the above, extracted once and reused

## Step 3 — unsafe places made predictable

- [ ] Every non-test `unwrap()`/`expect()` re-checked against the project rule (an
      adjacent proof it cannot fail) — audited: the real sites already carry one
- [ ] Poisoned-mutex handling consistent in production code (`unwrap_or_else(into_inner)`,
      never `unwrap()`), so one panicking worker cannot cascade
- [ ] Arithmetic that could overflow, and slice indexing that could panic, on inputs a
      user controls (file sizes, counts, byte offsets)
- [ ] Failure paths that currently swallow an error silently

## Step 4 — documentation brought up to date

- [ ] `docs/architecture/overview.md`: the header still says packaging is Windows-only
      and is dated 2026-09-06 — both untrue since the Linux packaging work
- [ ] `README.md`, `docs/__arch__/ROADMAP.md`, `docs/__arch__/open-questions.md`,
      `docs/architecture/invariants.md` re-read against the code as it is now
- [ ] `.ai/` rule modules and the generated `AGENTS.md` still accurate
- [ ] Every version-bearing sentence consistent with 2.0.1

## Step 5 — CI dependencies (the five Dependabot PRs)

Five open PRs, all raised by the Dependabot config added in the audit remediation. One of
them (`upload-artifact` 5 → 7) is what CI's own "Node.js 20 is deprecated" warning is
about, so these are not cosmetic — they are the difference between CI still working in
six months and CI breaking while nobody is watching.

- [ ] Each action re-pinned to the new version's commit SHA, in one commit
- [ ] The five Dependabot branches deleted, their PRs closed by the change landing
- [ ] Dependabot's own schedule reconsidered for a project about to go quiet

## Step 6 — release 2.0.1

- [ ] `Cargo.toml` workspace version → 2.0.1
- [ ] `apps/desktop/ui/package.json` → 2.0.1
- [ ] `CHANGELOG.md`: a real 2.0.1 entry saying what changed since 2.0.0
- [ ] `cargo xtask package` re-run: `setup.exe`/`SETUP.txt` must carry 2.0.1, or the
      gate's own installer-artifact check fails (by design — audit D-1)
- [ ] Tag `v2.0.1` pushed, so `release.yml` publishes the attested GitHub Release

## Step 7 — verification

- [ ] `cargo xtask gate` fully green locally (Windows leg)
- [ ] CI green on all three OS legs
- [ ] `package (linux)` green, including all three `install-and-run-linux` distro legs
- [ ] The release workflow's own run green, and the Release published

## Step 8 — completion

- [ ] Fast-forward merge into `main`, pushed
- [ ] Every remote branch but `main` gone
- [ ] Final report

## What this task will not do

Named up front so the report has nothing to confess later:

- **No behaviour changes.** A polish pass that changes what the product does is not a
  polish pass. Every item above is invisible to a user except the release itself.
- **The open questions stay open.** Q48 steps 2–4 (GPG signing, a Windows certificate),
  Q51 (the copy-step parallelisation measurement) and P-3's third part all need either
  the owner's key/money or a measurement run this session was asked not to make.
- **`codepack-desktop`'s glibc floor stays where it is.** Cross-building the whole
  webview stack against an older base image is a real project, not polish.
