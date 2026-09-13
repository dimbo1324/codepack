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

- [ ] Orientation: both feature branches finished, gate green, CI green on all three OS
- [ ] `main` fast-forwarded to the end of the chain (`main` → S13 → zoom is linear)
- [ ] This checklist committed **before** the work

## Step 1 — the version

- [ ] Root `Cargo.toml` and `apps/desktop/ui/package.json` to 2.1.0 — the only two places
      a version is written by hand now that `codepack-ai-api` inherits from the workspace
- [ ] `Cargo.lock` regenerated so the workspace crates carry the new number
- [ ] Nothing else claims 2.0.1 except history, which must keep it

## Step 2 — the documents

- [ ] `CHANGELOG.md`: the Unreleased section becomes 2.1.0, dated, written for someone
      using codepack rather than building it
- [ ] `README.md`: the current-release line
- [ ] `docs/architecture/overview.md`: date and version
- [ ] `docs/__arch__/ROADMAP.md`: the desktop zoom and monitor fit recorded — it is not a
      numbered stage, so it belongs where the other out-of-band additions are recorded
- [ ] `docs/__arch__/open-questions.md`: the release decision, and the zoom defects worth
      remembering
- [ ] Every version-bearing sentence consistent with 2.1.0

## Step 3 — rebuild and the installer

- [ ] `cargo xtask gate` green before packaging — the gate is what licenses a merge
- [ ] `cargo xtask package` builds the NSIS installer
- [ ] `setup.exe` and `SETUP.txt` at the repository root updated together, with the
      version, the build time, the commit and the checksum all describing **this** build
- [ ] The gate's `installer artifact` section re-run afterwards, since it is the check
      that compares `SETUP.txt` against the bytes beside it — including the `source:`
      line, which is the only field describing the binary rather than the build that
      copied it

## Step 4 — publish

- [ ] Fast-forward merge into `main`
- [ ] `main` pushed to `origin`
- [ ] Tag `v2.1.0` pushed, and `release.yml` watched to a conclusion rather than assumed
- [ ] CI green on all three OS legs for the merged `main`

## Step 5 — leave one branch

- [ ] Every local branch but `main` deleted
- [ ] Every remote branch but `main` deleted
- [ ] Verified from `git branch -a` rather than from memory

## Step 6 — completion

- [ ] Checklist filled with `+`/`-`
- [ ] Final report
