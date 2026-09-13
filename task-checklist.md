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

## Step 4 — publish

- [+] Fast-forward merge into `main`
- [+] `main` pushed to `origin`: `9a48eb1..72b7afc`
- [ ] Tag `v2.1.0` pushed, and `release.yml` watched to a conclusion rather than assumed
- [ ] CI green on all three OS legs for the merged `main`

## Step 5 — leave one branch

- [+] Every local branch but `main` deleted
- [+] Every remote branch but `main` deleted
- [+] Verified from `git branch -a` after `fetch --prune`: `main` and nothing else,
      local or remote

## Step 6 — completion

- [ ] Checklist filled with `+`/`-`
- [ ] Final report
