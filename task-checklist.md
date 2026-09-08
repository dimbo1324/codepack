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

- [x] G-1 application log file: `LogLine` with redaction in its constructor, sink on the
      existing progress channel, rotation/retention config, panic hook, `--collect-logs`,
      README documentation, a desktop settings toggle for verbosity
- [x] S-8 closed by the same mechanism (redaction moves into the send path — the `log`
      closure in `run_export`, the one place every step's narration funnels through)
- [x] D-1 mechanism: `xtask::installer`, gate step guarding staleness, `.gitattributes`,
      `.exportignore`, README, Q44 — the real `setup.exe`/`SETUP.txt` themselves are a
      separate commit at the very end (D-2 step 7: a binary must not share a diff with
      code, and any commit before the last one would make it stale again immediately)
- [x] G-2 gate run report: `xtask::gate_report`, per-section timing, JUnit XML for
      `tests`, `target/gate-logs/latest/` + 20-run rotation, `$GITHUB_STEP_SUMMARY`,
      CI artifact upload on failure — gate now runs every section instead of stopping
      at the first failure

## Step 5 — decisions resolved via safe interim default + recorded open question

- [x] Q-2 7z format declared one-directional, in help text/README/UI, and enforced by
      `ArchiveFormat::ensure_reopenable` before `verify`/`handoff`/desktop extraction —
      not just documentation (Q46; Q44 went to D-1 instead)
- [x] L-4 schema change (backslash → forward slash separator) recorded as Q45 (done
      earlier in Step 3, alongside L-3), not implemented this pass
- [x] L-9 case-insensitive directory matching on Linux documented as-is (Q47) — and the
      audit's own claimed workaround (`.exportignore` negation) was checked and found
      not to actually rescue a base-ignored name; corrected rather than repeated
- [x] S-6 GitHub Releases + build provenance attestation, no paid cert this pass (Q48) —
      decision recorded; implementation is Step 6's C-7, not this step
- [x] D-1 git-history-growth caveat — folded into Q44 itself rather than a separate
      entry, since the audit's own text asks for the decision and the caveat recorded
      together in one place
- [x] Q-5 legacy archive checked for the Russian header string (Q50) — unpacked
      `codepack-main.zip` and confirmed it is a port oversight, not parity: legacy's own
      `i18n.py` carries both languages and picks by config. Fixed, not just recorded:
      `write_text_dump` now follows `artifact_language` via `codepack_reports::i18n`

## Step 6 — security and supply chain

- [x] S-4 every third-party action in `ci.yml` pinned to a commit SHA (version as a
      comment), `.github/dependabot.yml` added for `github-actions`,
      `persist-credentials: false` on both `actions/checkout` steps; README's own
      example workflow fixed to demonstrate the same instead of contradicting it
      (`actions/checkout@v4`/`dimbo1324/codepack@main`, both moving refs)
- [x] S-5 mask world/group-write on extraction (`& !0o022`), plus the test the audit
      itself specified (cross-checked with `cargo check`/`clippy --target
      x86_64-unknown-linux-gnu`, since this dev machine is Windows; not exercised by a
      live run here — CI's Linux/macOS legs do that)
- [x] S-9 extracted bundles moved to `data_dir` (only changes anything on Linux, per
      the audit's own note that Windows/macOS already coincide), old location deleted
      at startup, 14-day/1024 MB retention sweep matching the scan cache's own shape
- [x] S-10 `cargo auditable build` replaces the plain build in
      `xtask::packaging_assets::prepare` (the Linux-shipped `/usr/bin/codepack`);
      verified locally that the resulting binary carries a `.dep-v0` section
      (`objdump -h`) `cargo audit bin` reads. CI installs `cargo-auditable` via
      `taiki-e/install-action`, matching `cargo-deny`. Scoped to this one binary, not
      the SBOM half (`cargo cyclonedx`/`cargo sbom`) or the Tauri-built desktop binary
      on Windows/macOS — Tauri's own internal `cargo build` call has no documented hook
      to substitute in `cargo auditable`, and guessing at one without testing a real
      Tauri release build would risk a silently broken installer for a low-priority
      finding; left as follow-up work, not silently dropped
- [x] S-12 `.github/workflows/ai-api-weekly.yml` — Monday cron, `cargo xtask ai-api`
      (format/clippy/test), same pinning/permissions discipline as `ci.yml`; verified
      the crate currently passes (27 tests) so this job starts green, not already red
- [x] C-7 `.github/workflows/release.yml`: on `v*`, builds Windows + Linux installers
      via the same `cargo xtask package` the gate already exercises, attests every
      artifact (`actions/attest-build-provenance`, S-6 Q48 step 1), publishes to a
      GitHub Release; the SHA256SUMS.txt-per-format-directory collision (verified by
      hand against a fake artifact tree) is resolved by renaming before upload, since
      release assets have no subdirectories

## Step 7 — performance, measured first

- [x] T-3/C-3 `.github/workflows/perf-smoke-weekly.yml` — Monday cron + `workflow_dispatch`,
      fixed `ubuntu-latest` runner (needed for the absolute-time backstop to be
      comparable week over week; the scaling ratio alone would tolerate different
      hardware), `--nocapture` so the numbers reach the log, one line per run appended
      to `$GITHUB_STEP_SUMMARY`, full log kept as a build artifact
- [x] Break perf_smoke's timing down per pipeline step — no new instrumentation inside
      `codepack-engine`: `timed_export` now drains the same `StepStarted`/`StepFinished`
      events both shells already consume, on a side thread, time-stamping each as it
      arrives; prints per-step duration and per-step scaling factor (5k → 50k) for each
      of the eight steps. Not run end-to-end on this dev machine — the user asked not to
      run the multi-minute `--ignored` benchmark interactively; verified by
      `cargo build`/`cargo clippy --all-targets` and code review, with the real
      execution left to the new scheduled job above
- [ ] P-4/Q-7 parallelize copy step if measurement shows it matters, else record why not
      — genuinely not done, honestly: the audit's own instruction is "measure first,
      then decide", and this session cannot supply that measurement — the multi-minute
      `--ignored` perf_smoke run was not executed here per explicit user preference
      (interactive long-running benchmarks avoided this session). The per-step timing
      breakdown that would answer "does copy actually dominate at 50k files" now exists
      (this Step 7 slice, committed) and the new `perf-smoke-weekly.yml` job will
      produce real numbers on its first scheduled run. Neither parallelizing copy
      blind nor recording a "measured, decided against it" rationale I do not have
      would be honest; recorded as open in Q51 instead of guessed either way
- [x] P-5 scan-cache mutex poisoning asymmetry fixed (`lookup`/`store` now recover from
      poisoning the same way `flush` already did); 2 new tests, verified to fail
      without the fix before committing it
- [x] P-3/Q-8 (part 1 of 3) observable WAL fallback: `codepack_storage::journal_mode`,
      surfaced by `codepack doctor` (JSON and human output), tested both ways (a
      freshly-created database reports `wal`; the field is absent, not `null`, when no
      database exists yet — opening one just to check would create it)
- [ ] P-3 (part 2 of 3) single shared `Mutex<Connection>` in desktop `AppState`, replacing
      a fresh connection per Tauri command — genuinely not done: needs careful
      lock-scoping so a long export's own writes do not serialise a concurrent
      `history` read behind them, which is a properly tested structural change, not one
      to rush near the end of an already large remediation pass. Recorded in
      `docs/architecture/overview.md`'s Known debt rather than silently dropped
- [ ] P-3 (part 3 of 3) a second `wal_concurrency` regression test from two real
      **processes**, not threads — deferred alongside part 2 for the same reason
- [x] P-9/Q-9 `codepack-security::scan`'s three `sort_by` calls (files/secrets/risky)
      converted to `sort_by_cached_key`, stability preserved and restated in comment
      (invariant I5); `codepack-scanner::walk.rs`'s `compare_like_os_walk` deliberately
      left alone — it is a `walkdir::sort_by` per-directory-level comparator, a
      structurally different API with no `sort_by_cached_key` equivalent, and each
      directory's own child count (not the whole tree's file count) is what actually
      bounds its cost
- [x] Q-11 `codepack-sanitize::format::path_lookup::find_on_path` memoized for the life
      of the process (`OnceLock<Mutex<HashMap<...>>>`), with the caveat documented that
      a formatter installed mid-run goes unnoticed until restart

## Step 8 — remaining tests and hygiene

- [x] The 15 adversarial tests from `04-TESTS.txt` not already covered above — audited
      one by one against everything already done this session:
      - Already covered by earlier work in this session: 2 (non-UTF-8 filename), 3
        (path check accepts own output), 5 (7z reads like ZIP, Q-2), 6 (extraction
        doesn't create a world-writable file, S-5), 9 (secret doesn't reach the
        progress channel, S-8)
      - Already covered before this session, confirmed still present: 1 (backslash
        filename), 4 (foreign path rejected), 7 (a hung formatter doesn't hang the
        run, P-1), 8 (watch doesn't subscribe to ignored dirs, L-2), 14 (`codepack` in
        the built package, CI `.deb` check), 15 (app starts without a tray, "GUI
        starts under Xvfb" CI step)
      - 13 (log directory respects `$XDG_STATE_HOME`): already adequately covered —
        `xdg_dir_from`'s 3 unit tests plus `linux_layout_matches_blueprint_d4`'s
        wiring test, confirmed against `resolve_base_dirs()`'s source calling
        `xdg_dir("XDG_STATE_HOME", ...)` distinctly from the `XDG_DATA_HOME` call — no
        new work needed
      - 10 (a huge text file doesn't eat memory): the test already existed
        (`crates/codepack-security/tests/large_file_scan.rs`, `#[ignore]`-gated, its
        own doc comment already said "run explicitly or in the weekly job") but
        `grep -rln large_file_scan .github/` found it wired into nothing — genuine gap,
        closed by adding it to `.github/workflows/perf-smoke-weekly.yml`
      - 12 (cancelling mid-packing of a large file is not interruptible): no test
        existed. Written as a characterization test (documents current known-debt
        behavior, does not fix it, per the audit's own instruction to write such a
        test before anyone starts fixing the behavior) —
        `crates/codepack-archive/tests/cancellation_mid_file.rs`, `#[ignore]`-gated,
        wired into the same weekly job. Verified by running it explicitly
        (`cargo test -p codepack-archive --release --test cancellation_mid_file --
        --ignored --nocapture`): passes, confirming both writers only check
        cancellation once per member and never mid-copy
      - 11 (two processes opening a new database simultaneously): explicitly and
        already deferred with reasoning in the P-3/Q-8 commit — recorded in
        `docs/architecture/overview.md`'s Known Debt, not silently dropped
- [x] T-11 `codepack-sanitize`'s gofmt/ruff/ktlint tests print `skipped: <tool> is not
      on PATH` via a shared `skip_if_absent` helper instead of returning silently;
      `gofmt` itself needs no new CI step since Go ships preinstalled on all three
      GitHub-hosted runner images, so that test already exercises the real path there
- [x] T-12 the real-repository tests in `network_isolation.rs`, `report_redaction.rs`,
      `ai_api.rs` and `ignored_advisories.rs` moved into their own
      `mod against_the_real_repository`, apart from the fixture-driven unit tests —
      `network_isolation.rs`'s two (from separate `tests`/`bypass_tests` modules) also
      lost a duplicated path-computation block to one shared `workspace_root()` helper
- [x] C-5 completions test stops flooding the gate log — `write_completions` split out
      so the test captures into a `Vec<u8>` instead of calling `run` (which writes to
      real stdout) directly; now asserts the script is non-empty and actually names
      `codepack` and a real subcommand, not just "did not panic". `manpage.rs`'s own
      test already captured correctly, so needed no change
- [x] C-6 `package-linux`/`install-and-run-linux` split out of `ci.yml` into a new
      path-filtered `.github/workflows/package-linux.yml` (`tauri.conf.json`, the
      Linux-relevant xtask sources, its own workflow file), plus `workflow_dispatch`;
      the release tag path needs no trigger of its own here since `release.yml`
      already runs the same `cargo xtask package` on every `v*` tag
- [x] C-8 `Swatinem/rust-cache` in the new `package-linux.yml` gets `prefix-key:
      "package-linux-release"` (distinct from `gate`'s ubuntu leg, a debug build) and
      `save-if: github.ref == 'refs/heads/main'` so a topic branch only reads the
      shared cache rather than filling it with a branch-specific variant
- [x] S-11 a relative `core.hooksPath` that climbs outside the repository (the audit's
      own `../../elsewhere` example) is now refused rather than followed — real bug,
      not just a missing test: `hooks_directory` reused `codepack_core::safe_join`, the
      same traversal check archive extraction already relies on, rather than a new
      bespoke one. Verified the new test fails without the fix (reverted locally,
      confirmed the hook wrote outside the tempdir, restored the fix) before
      committing. A negative control (nested-but-still-inside path) proves the fix
      does not just reject every path with a separator. The "tell the user the real
      path" half of the finding needed no change — `InitReport.hook`/`print_human`
      already showed the resolved path and flagged a custom `hooksPath`

## Completion

- [x] Full `cargo xtask gate` green locally (Windows leg) — 12/12 sections, re-verified
      after each fix below
- [x] Push and confirm CI green on all three OS legs and the packaging job — first push
      of this branch (needed to actually exercise CI, which this session's commits
      could only build/clippy/test locally for) found four real, previously-unexecuted
      bugs, all fixed and re-pushed, final state confirmed fully green:
      - macOS gate: `codepack-scanner`'s
        `a_real_non_utf8_named_file_on_disk_does_not_break_the_whole_plan` was
        `#[cfg(unix)]` but writes a genuinely non-UTF-8-named file to disk — Linux
        accepts that as an opaque byte string, macOS's APFS refuses it with `EILSEQ`
        before the test's own fixture setup can finish. Narrowed to
        `#[cfg(target_os = "linux")]`; the classification logic itself stays proven on
        every platform via a separate synthetic-path unit test. Documented in
        `.ai/project/15-command-reference.md` and `.ai/CHANGELOG.md`.
      - `package-linux.yml`'s `install-and-run-linux`: never actually run before this
        push (added in c9d233a, whose own commit message said so explicitly). Failed
        on all three distro images — `apt-get`/`dnf install` on an unexpanded glob
        that matched nothing, because `actions/upload-artifact`'s own documented
        behavior roots the artifact at the *least common ancestor* of its glob
        patterns (`target/release/bundle/`), not the original build path the
        `install-and-run-linux` matrix still named. Fixed by pointing
        `package_glob` at `deb/*.deb`/`rpm/*.rpm` — confirmed against
        `actions/upload-artifact`'s README rather than guessed.
      - `install-and-run-linux`'s debian:12 leg, third bug: with the two fixes above,
        the package installed correctly on all three distros, but `codepack export`
        through the installed CLI failed only on Debian 12 with exit 1. Root-caused
        with real evidence, not guessed: `objdump -T target/release/codepack` showed a
        weak reference to `pidfd_spawnp`/`pidfd_getpid` under `GLIBC_2.39`; running the
        actual binary in a real `debian:12` container reproduced
        `version 'GLIBC_2.39' not found` directly. Ubuntu 24.04 (this runner's own OS)
        ships glibc 2.39, Fedora 41 ships 2.40, Debian 12 ships only 2.36 — glibc
        promises forward compatibility, never backward, so building natively on the
        runner silently assumed a newer floor than the CI matrix's own oldest distro
        provides. Fixed by cross-building `codepack-cli` inside a `debian:12` container
        in both `package-linux.yml` and `release.yml` (the latter matters more: it is
        what a real user downloads) — verified afterward, in the same containers, that
        the rebuilt binary requires only up to `GLIBC_2.34`, still carries
        `cargo-auditable`'s `.dep-v0` section, and actually runs on `debian:12`,
        `ubuntu:24.04` and `fedora:41`. `xtask::packaging_assets::ensure_cli_binary`
        (new, unit-tested) accepts the pre-built binary via `CODEPACK_CLI_PREBUILT`
        instead of silently having `cargo xtask package` overwrite it with a native
        (broken) rebuild. Deliberately not applied to `codepack-desktop` in this pass —
        recorded as narrower, separate Known Debt in overview.md, since that check is
        `continue-on-error` and cross-building the whole webview stack is a bigger,
        separately-risked change.
      - Fourth bug, found immediately after the third fix landed: `package (linux) #4`
        failed at `cargo xtask package` with a bare "exit code 101" — but the GitHub
        REST API's per-step timestamps (`.../actions/runs/<id>/jobs`, unlike the
        public-but-sparse annotations) showed the step started and finished one second
        apart, right after all seven new debian:12 steps had genuinely succeeded. A
        real compile failing that fast was implausible, so measured what those seven
        steps actually leave behind: a from-scratch `cargo auditable build --release
        -p codepack-cli` alone puts 2.2 GiB of intermediate build output into
        `target/release`, because building the container with `/repo` bind-mounted
        wrote that output straight onto the runner's own disk — immediately before the
        native `cargo xtask package` step needed to compile much of that same
        dependency graph a second time (a different toolchain path) for the Tauri
        desktop build. A GitHub-hosted runner's default disk is roughly 14 GiB; two
        overlapping multi-GiB compilations exhausting it before the second one writes
        its first byte matches an instant failure exactly. Fixed with
        `CARGO_TARGET_DIR=/build` — a path inside the container's own ephemeral
        filesystem, never bind-mounted — so the container's own output disappears with
        it (`docker rm -f`) instead of ever touching the runner's disk; only the
        finished binary crosses back out via `docker cp`. Verified locally: the
        isolated build succeeds, the host's `target/release` does not grow at all
        (2.2 GiB before and after), and the copied-out binary still runs correctly in
        `debian:12`/`ubuntu:24.04`/`fedora:41`.
      - Re-pushed after each fix. Final push (commit `f8b429b`) confirmed fully green
        via the GitHub API: `CI` (all three OS legs) `success`; `package (linux)`
        `success`, including every one of `cargo xtask package`, `Inspect the .deb`,
        `Verify the checksums`, the artifact upload, and all three
        `install-and-run-linux` distro legs (`ubuntu:24.04`, `debian:12`, `fedora:41`)
        — install, headless export, and the Xvfb GUI-start check all `success`.
- [ ] `docs/architecture/overview.md`, README, ROADMAP `**Status.**` lines updated
- [ ] No dead code, no magic numbers left unexplained, duplication from the audit resolved
- [ ] `cargo xtask package` run; `setup.exe`/`SETUP.txt` reflect the final state
- [ ] Fast-forward merge into `main`, push to `origin/main`
- [ ] Final report naming everything done, everything deferred with its open-question number,
      and everything that could not be verified from a Windows machine (Linux runtime behavior
      is proven only by CI, exactly as the audit itself could only do)
