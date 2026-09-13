# Task Checklist

**Task:** Prepare the project for use and publish it. Update the documents, rebuild,
produce a current installer, merge everything into `main`, push, and leave `main` as the
only branch.

**Date:** 2026-09-13
**Branch:** `chore/release-2.1.0`

Owner instruction, 2026-09-13: «подготовь все к работе проекта. Обнови доки,
перекомпилируй, сделай актуальные exe и влей на удаленный репо. также сделай слияние с
main. Удали все ветки кроме main.»

This is the explicit publish authorisation the workflow rules require: pushing to
`origin/main` happens only when the owner asks for it within the task, and they have.

## The version decision

The tree carried **two features under a released version number**: stage S13's API path
and the desktop's zoom and monitor fit, both merged but both still labelled 2.0.1.
Building `setup.exe` from that would have repeated precisely the defect the 2.0.1 task
found — an installer whose number describes one binary while its bytes are another,
except this time the difference would include a feature that reaches the network.

**Owner decision, 2026-09-13: 2.1.0, with the tag.** A minor bump: two additions, nothing
breaking. The tag is what `release.yml` watches, so this is a public GitHub Release with
every artifact and a build-provenance attestation, not merely a file in the repository.

## Step 0 — preparation

- [+] Orientation: both feature branches finished, gate green, CI green on all three OS
- [+] `main` fast-forwarded to the end of the chain (`main` → S13 → zoom is linear)
- [+] This checklist committed **before** the work (`382c9d3`)

## Step 1 — the version

- [+] Root `Cargo.toml` and `apps/desktop/ui/package.json` to 2.1.0 — the only two places
      a version is written by hand now that `codepack-ai-api` inherits from the workspace
- [+] `Cargo.lock` regenerated; `cargo metadata` reports a single version across every
      workspace crate, which is how it was checked rather than by reading two files
- [+] Nothing else claims 2.0.1 except history, which must keep it — swept with grep

## Step 2 — the documents

- [+] `CHANGELOG.md`: the Unreleased section becomes 2.1.0, dated, and its "not done"
      part now names all three unverified things plainly rather than burying them
- [+] `README.md`: the current-release line
- [+] `docs/architecture/overview.md`: date and version
- [+] `docs/__arch__/ROADMAP.md`: the zoom work recorded as an addition to S11, with the
      three defects only running the application could find; the release itself beside
      2.0.0 and 2.0.1 under S14
- [+] `docs/__arch__/open-questions.md`: the release decision, why "bump but do not tag"
      was declined, and what remains unverified
- [+] Every version-bearing sentence consistent with 2.1.0

## Step 3 — rebuild and the installer

- [+] Gate run before packaging: 10 of 11 sections green and `installer artifact`
      **failed**, which is the check doing its job — `SETUP.txt` still said 2.0.1 while the
      project said 2.1.0
- [+] `cargo xtask package` built `codepack_2.1.0_x64-setup.exe`
- [+] `setup.exe` and `SETUP.txt` updated together: version 2.1.0, built
      2026-09-13T07:37:38Z, commit `382c9d3`, and the checksum **verified independently**
      against the file with `Get-FileHash` rather than taken from the build's own word
- [+] Full gate re-run on a **clean** tree: 11/11 green in 96s, `installer artifact`
      included. The `source:` line names `codepack_2.1.0_x64-setup.exe`, which is the
      field that describes the bytes rather than the build that copied them

## What the first release run found — the reason this took two attempts

The first `v2.1.0` tag's `release.yml` failed on the `debian:12` cross-build of
`codepack-cli`: it had just gained `codepack-ai-api`, whose `ureq` pulled OpenSSL, and the
container had no headers. **No GitHub Release was published** — `publish release` was
skipped — and CI for `main` itself was green on all three OS.

Fixing that uncovered the real defect, which no test and no review had caught:

- [+] **S13's API path could never have worked.** `ureq` gates TLS provider selection on
      `cfg(feature = "native-tls")`; the manifest enabled `native-tls-no-default`, which
      adds the dependency without selecting it. Every HTTPS request panicked, on every
      platform, since 2026-07-27. Reproduced in a real `debian:12` container
- [+] **Owner decision: rustls with roots from the OS trust store.** Both original
      requirements hold — the platform store (corporate proxies) and no CDLA Mozilla list
- [+] `platform-verifier` tried first and **rejected**: it depends on `webpki-root-certs`,
      invisible on Linux and caught only by `cargo deny` reading every target. My report to
      the owner had claimed that route was CDLA-free; that was wrong and was corrected
- [+] Roots loaded directly via `rustls-native-certs` (Apache-2.0 OR ISC OR MIT) as
      `RootCerts::Specific`. `cargo deny` passes with **no new exception**; `openssl-sys`,
      `webpki-roots` and `webpki-root-certs` are absent on every target
- [+] **Proven by a real handshake** with `api.anthropic.com` — on Windows, and in
      `debian:12` three runs in a row (one earlier container run failed once, transiently,
      before the network warmed up; recorded rather than hidden)
- [+] `tests/tls_handshake.rs`: the handshake `#[ignore]`d and scheduled in
      `perf-smoke-weekly.yml`; the trust-store half always on
- [+] The `libssl-dev` workaround first added to both workflows **removed** again — no
      OpenSSL in the graph means none is needed
- [+] `package-linux.yml`'s path filter widened to manifests and the lockfile: the branch
      that broke packaging touched none of the filtered paths
- [+] Installer rebuilt from `c41eaa9`, which contains the fix: 6.6 MiB, built
      2026-09-13T11:49:34Z, checksum verified independently. The gate's installer check
      had passed against the *pre-fix* installer — version and checksum agreed with each
      other while describing a binary that could not make a request — so rebuilding was
      decided by reading the commit, not by the check
- [-] **A complete exchange with a key** is still unverified. The connection is proven;
      question-and-answer needs a real API key, which this session does not have and must
      not handle

## Step 4 — publish

- [+] Fast-forward merge into `main`
- [+] `main` pushed to `origin`: `9a48eb1..72b7afc`
- [ ] Tag `v2.1.0` moved to the fixed commit and pushed — owner decision, since nothing was
      ever published under the first one — and `release.yml` watched to a conclusion
- [ ] CI green on all three OS legs for the merged `main`

## Step 5 — leave one branch

- [+] Every local branch but `main` deleted
- [+] Every remote branch but `main` deleted
- [+] Verified from `git branch -a` after `fetch --prune`: `main` and nothing else,
      local or remote

## Step 6 — completion

- [ ] Checklist filled with `+`/`-`
- [ ] Final report
