# Task Checklist

**Task:** Final polish before the project is handed over for use and left alone for about
six months: remove unused code, collapse remaining duplication into shared entities, make
the unsafe-looking places predictable, bring every document up to date, clear every
remote branch but `main`, cut release 2.0.1, refresh the build, and re-run everything.

**Date:** 2026-09-08
**Branch:** `chore/2.0.1-final-polish` (merged fast-forward into `main`, then deleted)

Owner instruction, 2026-09-08: plan first, then work through it, and report once done.

## The shape of this task

This is not a feature pass. Everything here either **removes** something (dead code,
duplication, a stale sentence, a branch) or **makes an existing thing more predictable**
(a recompiled regex, an unhandled failure mode). The one addition is the release itself.

The bar for touching code: it must be provable by test or by the gate. Nothing is
"cleaned up" on taste alone — this project is about to sit untouched for months, and a
refactor nobody will re-verify is a liability, not polish.

## Step 0 — preparation

- [+] Orientation ritual: git state, ROADMAP, overview, previous checklist, open questions
- [+] Branch `chore/2.0.1-final-polish` off green `main`
- [+] This checklist committed **before** the work started (commit `c270a90`)

## Step 1 — unused and dead code

- [+] Compiler-proven dead code: `cargo clippy --workspace --all-targets` clean with
      `-W dead_code -W unused_imports`. The Linux cross-check could **not** run here —
      `cc` needs `x86_64-linux-gnu-gcc` for the vendored C in git2/rusqlite, which this
      Windows machine does not have — so Linux-only dead code is proven by CI's own
      Ubuntu leg instead, which is green
- [+] Unused dependencies: every member's manifest compared against what its sources
      actually reference. Four removed: `serde` and `serde_json` (codepack-ai),
      `walkdir` (codepack-security dev-deps), `thiserror` (codepack-desktop). The
      workspace still builds and all tests pass, which is the proof they were dead
- [+] `#[allow(dead_code)]`: one site, `codepack-engine/tests/support/mod.rs`, legitimate
      for a shared test helper module not every test file uses. Left as is
- [+] `clean-project` now sweeps `crates/codepack-ai-api/target/` — 770 MB it could not
      see, because that crate is excluded from the workspace and builds into a target
      directory of its own. `selftest` green after the change

## Step 2 — duplication collapsed into shared entities

- [+] **Regexes recompiled per call.** 16 patterns across six files in `codepack-reports`
      were built by `fn x_pattern() -> Regex` called from inside the per-file loop.
      `code_quality.rs` already had the fix *and the reasoning written out*; the rest
      never got it. All 16 now use the same `static X: LazyLock<Regex>` shape:
      graph.rs (4), api_surface.rs (5), backend.rs (2), frontend.rs (2), key_files.rs (2),
      scripts.rs (1). Behaviour identical by construction and proven so by the crate's
      208 tests plus the golden comparison against the legacy implementation
- [+] Deliberately left alone: the `Regex::new` calls in codepack-security and
      codepack-scanner are all inside `#[cfg(test)]`, where they are the independent
      reference the hand-written scans are checked against

## Step 3 — unsafe places made predictable

- [+] Every non-test `unwrap()`/`expect()` re-checked: all carry an adjacent proof they
      cannot fail, as the project rule requires. Nothing to change
- [+] Poisoned mutexes: every production `lock()` already recovers with
      `unwrap_or_else(into_inner)`. The three bare `unwrap()`s found are all in test code
- [+] Slice indexing: ~45 byte-offset slices in codepack-security, every one derived from
      `str::find`, a match span, or an ASCII literal's length — read individually and
      confirmed to be valid boundaries by construction
- [+] **That confirmation now has a test.** `tests/utf8_boundaries.rs` runs redaction and
      a full project scan over Cyrillic, CJK, emoji and mixed content in every position
      around a secret, including pressed directly against it with no separator. Reading
      proves it today; the test proves it in six months. All three cases pass
- [-] Arithmetic overflow: no unguarded unsigned subtraction found on user-controlled
      values, so nothing was changed. Not an exhaustive audit of every numeric path —
      the search was pattern-based

## Step 4 — documentation brought up to date

- [+] `docs/architecture/overview.md`: date and version refreshed, and the header now
      records that Linux packages are installed and run in real distributions before
      shipping. (The "packaging is Windows-only" sentence had already been fixed in the
      audit remediation — the stale copy seen while planning was `main`'s, before that
      work merged)
- [+] **`docs/architecture/invariants.md`: all nine invariants now document how they are
      enforced.** Only I1 and I2 did before. Each addition names the real gate step,
      boundary type or test, and every referenced path and test name was checked to
      exist. This is the item that matters most for a project going quiet: it tells
      whoever opens it next which tests are load-bearing
- [+] `README.md` current release line moved to 2.0.1
- [+] `CHANGELOG.md`: a real 2.0.1 entry written for someone using codepack
- [+] `.ai/` modules and generated `AGENTS.md` verified in sync by the gate
- [+] Every version-bearing sentence consistent with 2.0.1

## Step 5 — CI dependencies (the five Dependabot PRs)

- [+] All five applied in one commit, 17 pins across five workflow files, each SHA
      resolved with `git ls-remote` rather than copied from a PR body: checkout
      5.1.0→7.0.1, upload-artifact 5.0.0→7.0.1, setup-python 6.3.0→7.0.0,
      attest-build-provenance 3.0.0→4.2.2, pnpm/action-setup 6.0.10→6.1.0. This also
      clears CI's standing "Node.js 20 is deprecated" warning
- [+] All five Dependabot branches deleted; all five PRs closed
- [+] Dependabot moved from weekly-per-action to **monthly, grouped, limit 3** — its first
      week under the old config produced five PRs and five branches, which is wrong for a
      repository read twice a year

## Step 6 — release 2.0.1

- [+] `Cargo.toml` workspace version → 2.0.1
- [+] `apps/desktop/ui/package.json` → 2.0.1
- [+] `crates/codepack-ai-api/Cargo.toml` → 2.0.1 — it spells its version out because an
      excluded package cannot inherit `version.workspace = true`, and **nothing compared
      the two until now**. New test in xtask's `ai_api` module asserts they match;
      verified to fail on a real drift before being kept
- [+] `CHANGELOG.md` 2.0.1 entry
- [+] `cargo xtask package` re-run — and this is where the release found a real defect,
      see the section below
- [+] Tag `v2.0.1` pushed; `release.yml` publishing the attested GitHub Release

## The defect this release found, which was not in the plan

`cargo xtask package` **published the previous version's installer.** Its own output said
so: "Finished 1 bundle at `codepack_2.0.1_x64-setup.exe`" followed by "published from
`codepack_2.0.0_x64-setup.exe`". The file a user would download as 2.0.1 was the 2.0.0
binary, with `SETUP.txt` declaring 2.0.1 over it.

`tauri build` names its output after the version and never removes the previous one, so
after a bump the directory holds both; `publish` took whichever the filesystem listed
first. The gate could not see it: it compares `SETUP.txt`'s version against `Cargo.toml`
(2.0.1 = 2.0.1) and `setup.exe`'s checksum against `SETUP.txt`'s (both taken from the
same stale file). Both halves agreed with each other while describing the wrong binary —
a check whose inputs all derive from one mistake cannot detect that mistake.

Fixed on both sides: `pick_installer` selects by version-in-filename and fails naming what
it found instead, and the gate now also checks the recorded `source:` line, which is the
only field describing the *bytes* rather than the build that copied them. Three tests,
including the regression proper. `setup.exe` rebuilt and genuinely 2.0.1.

- [+] Defect found, fixed, tested, and the mechanism that missed it strengthened

## Step 7 — verification

- [+] `cargo xtask gate` fully green locally, 12/12 sections
- [+] CI green on all three OS legs
- [+] `package (linux)` green, including all three `install-and-run-linux` distro legs
      (ubuntu:24.04, debian:12, fedora:41)
- [+] Release workflow run and the published Release verified

## Step 8 — completion

- [+] Fast-forward merge into `main`, pushed
- [+] Every remote branch but `main` gone: five Dependabot branches and this task's own
- [+] Final report

## What this task did not do

As named in the plan, and still true:

- **No behaviour changes.** Nothing a user does with codepack works differently. The one
  user-visible change is that `setup.exe` is now actually the version it claims to be
- **The open questions stay open.** Q48 steps 2–4 (GPG signing, a Windows certificate),
  Q51 (the copy-step parallelisation measurement) and P-3's third part all need either
  the owner's key/money or a measurement run this session was asked not to make
- **`codepack-desktop`'s glibc floor stays where it is.** The CLI is cross-built against
  Debian 12; the desktop binary is not, and cross-building the whole webview stack is a
  real project rather than polish
- **The Linux dead-code cross-check ran on CI, not here.** No `x86_64-linux-gnu-gcc` on
  this machine, so `cargo clippy --target x86_64-unknown-linux-gnu` cannot link the
  vendored C dependencies
