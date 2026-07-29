# Task Checklist

**Task:** Bring the application to a usable release on this branch, per the owner's
instruction of 2026-07-29. Three parts, one branch:

- **R1** — sanitize must be able to produce a **`.7z` archive** of the sterile copy at a
  path the user names. Today it only writes a folder, so the owner has to create a
  destination folder by hand every time and then archive it himself.
- **R2** — short release audit: find and fix real defects, remove dead code, no scope
  hunting beyond what an audit turns up.
- **R3** — release documentation: `README.md` currently claims "продуктового кода пока
  нет", which is false and is the first thing a new user reads.

**Date:** 2026-07-29
**Branch:** feat/budget-by-model-explain-pr-preset (continued after B1/B2/B3 landed as
`8caf9db`)

Owner approved, at the end and only once everything is green: push to `origin`, merge
into `main`, delete every branch except `main`.

**Scope boundary stated up front.** This is a *usable* release, not ROADMAP stage S14.
Code signing, notarisation, `SHA256SUMS.txt` and auto-update stay in S14 and are **not**
done here; the NSIS installer (`cargo xtask package`) already exists and is what ships.
Saying so now so the final report is not the first place it appears.

## Preparation

- [+] B1/B2/B3 finished, reviewed, gate green, committed as `8caf9db`
- [+] Review findings from that slice addressed before continuing (diff selection in
      `explain`, `skipped_dirs` parsing, case-folded path matching, test honesty)
- [ ] Orientation re-read for the new scope: `ROADMAP.md` §1, `BLUEPRINT.md` on the
      sterile copy, `docs/decisions/open-questions.md` Q24
- [ ] Decide where 7z lives before writing code (architecture, not after)

## R1 — a `.7z` archive of the sterile copy

- [ ] **Dependency decision, justified in the final report**: 7z cannot be produced by
      anything already in the tree (`zip` writes ZIP only). Evaluate `sevenz-rust2`
      (pure Rust, Apache-2.0, actively maintained fork of `sevenz-rust`) against the
      dependency rules in `.ai/universal/05-security-and-secrets.md`
- [ ] **Placement**: archiving belongs to `codepack-archive`, not to `codepack-sanitize`
      — dependencies point downward (`sanitize` → `archive` → `core`), no cycle, and the
      sterile copy does not become a second place that knows how to write archives
- [ ] `codepack-archive`: a `sevenz` module that packs a directory into one `.7z`,
      streaming rather than reading whole files into memory
- [ ] `SterileCopyOptions` gains `archive_path: Option<PathBuf>`; `None` keeps today's
      behaviour byte for byte (folder only), so no existing caller changes meaning
- [ ] The archive is produced **after** the folder and its `STERILE_COPY_REPORT` exist,
      and contains both — the report describes what is in the archive
- [ ] Invariant I2: the destination-overlap guard must also reject an archive path
      inside the source project
- [ ] Invariant I3: nothing goes into the archive that did not already pass redaction
      and the safety filter — the archive packs the destination folder, never the source
- [ ] CLI `sanitize --archive <path.7z>`; **`--out` becomes optional when `--archive` is
      given**, using a temporary folder that is removed afterwards. This is the actual
      complaint: "мне приходится создавать отдельную папку"
- [ ] Desktop: `start_sanitize` takes an optional archive path; `SterileCopyPage` gets a
      field and a picker, matching how the export destination is chosen
- [ ] Tests: archive is created and is a readable 7z; it round-trips (extract, compare);
      `None` produces exactly what it produces today; archive-inside-source is refused;
      `--archive` without `--out` leaves no stray folder behind; a cancelled run writes
      no archive

## R2 — release audit

- [ ] `unwrap()`/`expect()` outside tests without a proven-invariant comment
- [ ] `TODO`/`FIXME` that are not the tracked `TODO(cross-platform)` markers
- [ ] Dead code: `#[allow(dead_code)]`, unused public API, modules nothing calls
- [ ] Stale documentation inside the code (module docs that describe old behaviour)
- [ ] Independent review pass (`codepack-quality-reviewer`) over the whole branch diff
- [ ] Fix what the audit finds; anything deliberately not fixed is named in the report

## R3 — release documentation

- [ ] `README.md` (Russian, owner-facing) rewritten: what the product is, install, the
      commands that exist, the desktop app, where artifacts land, the privacy guarantee.
      It currently says there is no product code, which is false
- [ ] `ROADMAP.md` — `**Status.**` lines for what this branch shipped; §1 status column
- [ ] `docs/architecture/overview.md` — `codepack-archive`, `codepack-sanitize`,
      `codepack-cli`, desktop rows
- [ ] `docs/decisions/open-questions.md` — the 7z decision and anything it surfaces
- [ ] `.ai/` rule modules only if this task changed how work is done

## Verification (at the very end, all parts together)

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo xtask gate` (full)
- [ ] `cargo deny check` accepts the new dependency and its transitive tree
- [ ] `cargo xtask package` produces an installer
- [ ] Manual smoke: sanitize a real project to a `.7z` and open it

## Completion

- [ ] Checklist filled `+`/`-` honestly, final report in Russian
- [ ] Push to `origin`, fast-forward merge into `main`, delete every branch but `main`

---

## Next task

Not yet chosen. Start with the orientation ritual from
`.ai/project/13-progress-tracking.md`.
