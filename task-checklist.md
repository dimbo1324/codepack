# Task Checklist

**Task:** Remediate every finding from `AUDIT-2026-09-07/` (67 findings across Linux,
security, code quality, tests, concurrency/performance, CI) plus the two owner-requested
features (application logging, a root-level installer duplicate), in the order set out in
`AUDIT-2026-09-07/09-PLAN.txt`.

**Date:** 2026-09-07
**Branch:** fix/audit-2026-09-07-remediation

Owner instruction, 2026-09-07: fix everything the audit found, in a working branch, with
microcommits step by step; run the full gate and tests at the end; update documentation;
remove dead code, magic numbers and duplication; rebuild the installer so the `.exe` is
current; merge to main and push; then shut down the machine. Work unhurried, double-check,
debug and test along the way.

## How this checklist is organized

Mirrors `AUDIT-2026-09-07/09-PLAN.txt`'s eight steps. Each finding code (L-, S-, Q-, T-,
P-, C-, G-, D-) is checked off individually. Where the audit's own plan calls for an
owner decision before code, the interim safe default from the audit is implemented and the
open question is recorded in `docs/__arch__/open-questions.md` instead of blocking — this
mirrors the audit's own Step 5 guidance and the project's rule that a blocking rule
conflict gets escalated, not guessed past silently.

## Step 0 — non-behavior-changing corrections

- [x] Q-6 README / overview.md / xtask USAGE / ROADMAP §1 status line: Linux packaging is real
- [x] S-3 deny.toml: rewrite the eleven GTK3 ignore reasons, add a revisit date
- [x] Q-1 (partial) fix the two false comments in export/mod.rs — superseded: Step 1's
      `ValidatedResultPath` mechanism rewrote both comments as part of closing S-1 itself,
      rather than as a separate stopgap patch first
- [x] C-2 (partial) add `permissions: contents: read` to ci.yml
- [x] L-12/S-6 (partial) one line in README about unsigned Linux packages

## Step 1 — broken and claimed (the critical findings)

- [x] T-2/S-7 test isolation for `AppPaths` — not the planned `CODEPACK_HOME` env-var
      override: `std::env::set_var` is `unsafe fn` on this toolchain (a genuine
      multi-threaded soundness hazard) and the workspace forbids `unsafe` outright, so
      every touched command was split into a dependency-injected `_at(paths: &AppPaths,
      ...)` form instead, tested against `AppPaths::for_root(&tempdir)`
- [x] S-2 fix `resolve_export_result`'s `LIMIT 0` query — replaced with
      `export_run_result_path_matches` (`LIKE` pre-filter + canonicalized comparison, no
      full-table scan)
- [x] T-1 acceptance-path tests: path accepted when a run produced it; all 6 commands reject a stranger path
- [x] S-1 `ValidatedResultPath` newtype; `extract_validated_bundle` becomes its method
- [x] Record the "boundary you can bypass gets a type or a gate step" rule in `.ai/universal/04-architecture-boundaries.md`

## Step 2 — process-halting / crashing risks

- [x] P-1 sanitize formatter: write stdin on its own thread, add timeout, honor cancellation
- [x] P-2/Q-3 shared file-read-size ceiling in codepack-core; scan reports partial-scan instead of silent skip
- [x] C-1 fix `.deb` inspection step in CI: read full output before matching, no `grep -q` under `pipefail`
- [x] L-5 tray build failure downgraded to a warning, does not abort startup

## Step 3 — Linux brought to a working state

- [x] C-4/L-13/T-9 CI job: install built package in ubuntu/debian/fedora containers, run headless export
      (GUI-under-Xvfb step is `continue-on-error`: exploratory for L-5/L-7, unverified by a live run — see final report)
- [x] L-1/L-10 CLI + shell completions + man page bundled into deb/rpm
- [x] L-2 watcher subscribes only to non-ignored directories; ENOSPC reported with guidance
      (thread-shutdown correctness reasoned through carefully, not exercised by a live Tauri app — see final report)
- [x] L-3/L-4 non-UTF-8 / backslash-named files excluded with a clear reason instead of silently breaking the baseline; open question recorded for the larger schema change (Q45)
- [x] L-6 `$XDG_STATE_HOME` respected for the log directory; the `logs`-subdirectory form question recorded as Q49 (BLUEPRINT §D.4 already fixes the current form)
- [x] L-8 Linux font names added to the stacks (unverified by a screenshot on a real GTK/fontconfig system — see final report)
- [x] L-7 WebKitGTK DMABUF workaround documented in README; the conditional code workaround is deferred per the plan's own sequencing (needs 3.1's containerized job to actually run and show whether it reproduces, which this session cannot observe)

## Step 4 — owner-requested features

- [ ] G-1 application log file: `LogLine` with redaction in its constructor, sink on the
      existing progress channel, rotation/retention config, panic hook
- [ ] S-8 closed by the same mechanism (redaction moves into the send path)
- [ ] D-1 `setup.exe` + `SETUP.txt` duplicate in repo root, gate step guarding staleness,
      `.gitattributes`, `.exportignore`
- [ ] G-2 gate run report: per-section timing, JUnit XML, `$GITHUB_STEP_SUMMARY`

## Step 5 — decisions resolved via safe interim default + recorded open question

- [ ] Q-2 7z format declared one-directional in help text/README/UI (Q44)
- [ ] L-4 schema change (backslash → forward slash separator) recorded as Q45, not implemented this pass
- [ ] L-9 case-insensitive directory matching on Linux documented as-is (Q46)
- [ ] S-6 GitHub Releases + build provenance attestation (no paid cert this pass) (Q47)
- [ ] D-1 git-history-growth caveat recorded alongside the decision (Q48)
- [ ] Q-5 legacy archive checked for the Russian header string; decision recorded

## Step 6 — security and supply chain

- [ ] S-4 pin GitHub Actions by SHA, add Dependabot for github-actions, `persist-credentials: false`
- [ ] S-5 mask world/group-write on extraction (`& !0o022`)
- [ ] S-9 extracted bundles moved to `data_dir`, given a ceiling and a retention sweep
- [ ] S-10 `cargo auditable` in the packaging step
- [ ] S-12 weekly scheduled job building `codepack-ai-api`
- [ ] C-7 release job on tag

## Step 7 — performance, measured first

- [ ] T-3/C-3 perf_smoke runs on a schedule
- [ ] Break perf_smoke's timing down per pipeline step
- [ ] P-4/Q-7 parallelize copy step if measurement shows it matters, else record why not
- [ ] P-5 fix scan-cache mutex poisoning asymmetry (one-line fix, do regardless of measurement)
- [ ] P-3/Q-8 observable WAL fallback in `doctor`; single shared connection in desktop `AppState`
- [ ] P-9/Q-9/Q-11 `sort_by_cached_key`, formatter PATH lookup cache

## Step 8 — remaining tests and hygiene

- [ ] The 15 adversarial tests from `04-TESTS.txt` not already covered above
- [ ] T-11 print skip reason instead of silently passing when a tool is absent
- [ ] T-12 split unit tests from against-the-real-repository tests in xtask
- [ ] C-5 completions test stops flooding the gate log
- [ ] C-6 package-linux job scoped to relevant paths
- [ ] C-8 rust-cache prefix keys for package-linux
- [ ] S-11 test for `core.hooksPath` pointing outside the repo

## Completion

- [ ] Full `cargo xtask gate` green locally (Windows leg)
- [ ] Push and confirm CI green on all three OS legs and the packaging job
- [ ] `docs/architecture/overview.md`, README, ROADMAP `**Status.**` lines updated
- [ ] No dead code, no magic numbers left unexplained, duplication from the audit resolved
- [ ] `cargo xtask package` run; `setup.exe`/`SETUP.txt` reflect the final state
- [ ] Fast-forward merge into `main`, push to `origin/main`
- [ ] Final report naming everything done, everything deferred with its open-question number,
      and everything that could not be verified from a Windows machine (Linux runtime behavior
      is proven only by CI, exactly as the audit itself could only do)
