# Task Checklist

**Task:** Three security-value features proposed and approved 2026-07-29, all in one
branch (owner instruction):

- **A1** — `codepack verify <bundle>`: re-scan an already-produced bundle (ZIP, archive
  set, or extracted folder) and answer "clean / here is what is in it". Turns the
  product's central promise from "trust the pipeline" into something the **recipient**
  can check, not only the producer.
- **A2** — `.codepack-allow`: a project-level list of reviewed findings, so a known
  false positive stops being re-reported every run. Alert fatigue is how a real finding
  gets ignored. Fingerprint = (rule, file, hash of the **already-redacted** message) —
  never the secret itself (invariant I3).
- **A3** — `codepack scan --staged`: scan only what is staged in git, making the product
  usable as a real pre-commit guard in someone else's repository — prevention at the
  source, not only safe handoff.

**Date:** 2026-07-29
**Branch:** feat/verify-allowlist-staged-scan

Owner explicitly approved, at the end of this task and only once everything is green:
push to `origin`, merge into `main`, delete every branch except `main`.

## Preparation

- [+] Orientation: git status/log, `ROADMAP.md` §1, `docs/architecture/overview.md`,
      `task-checklist.md` (previous task closed cleanly), `docs/decisions/open-questions.md`
- [+] Confirmed by reading the code that none of the three already exist: CLI commands
      are `export`/`preview`/`scan`/`history`/`doctor`/`sanitize`/`completions`;
      `ScanArgs` carries only the shared `ProjectArgs`; no allowlist/baseline anywhere
- [+] Reusable pieces confirmed: `codepack_archive::extract_zip_safely`/
      `restore_archive_set` (S8, path-traversal-safe), `codepack_security::scan_project`
      (S3), `output::Envelope`/`Format` (S10), `exit::Outcome` contract (0/1/2/3),
      `ProjectConfig` (`.codepack.toml`) as the precedent for a project-level TOML file

## A1 — `codepack verify <bundle>`

- [+] `cli.rs`: `Verify(VerifyArgs)` with a positional bundle path; accepts a `.zip`, an
      archive-set directory (`ARCHIVE_SET_MANIFEST.json` present), or an
      already-extracted bundle folder — decided by inspection, not by a flag
- [+] `commands/verify.rs`: extract (ZIP/set → temp dir via `extract_zip_safely`, which
      already refuses traversal entries) or use the folder as-is, then `scan_project`
      over the extracted contents; temp dir removed on every exit path
- [+] Decide and record: whether findings inside the bundle's own `reports/` tree are
      reported, suppressed, or reported-but-labelled. **Was skipped in the first pass
      and caught by independent review** — the command shipped reporting ~24 findings
      for a provably clean bundle. Resolved afterwards by exporting a real bundle and
      reading what the scanner actually did: findings are now *reported-but-labelled*,
      split into "exported content" (drives the verdict) and "codepack's own generated
      reports" (printed, counted, out of the verdict). Two signals decide it — a known
      generated-artifact path, or a bundle line with no credential-shaped run left
      after redaction markers are stripped. Recorded as Q27.
- [+] Exit codes reuse the existing contract: 0 clean, 3 critical findings present,
      1 real failure (unreadable/corrupt archive) — a real failure outranks 3
- [+] `--json` via the existing envelope (`command: "verify"`), plus a
      `generated_findings` array so the second group is machine-readable too
- [+] Tests: clean bundle → 0; bundle with a planted secret → 3 and the finding is
      reported; corrupt/missing archive → 1 with a readable error. The clean-bundle
      test originally asserted only `critical == 0`, which passed while 24 findings
      were reported — strengthened by review to assert the content findings really are
      empty, which is what the command's name promises.
- [-] A traversal-hostile ZIP refused, asserted at the *command* level: not added.
      `codepack-archive` already proves this at the library level with a hand-built
      malicious archive (S8, `tests/security.rs`), and `verify` calls exactly that
      function; a second copy of the fixture would assert the same guarantee twice.

## A2 — `.codepack-allow`

- [+] `codepack-core`: new `allowlist` module next to `config::project` (same TOML +
      `deny_unknown_fields` + "unknown key is an error" precedent as `.codepack.toml`,
      Q6). Fingerprint helper takes (rule, file, redacted message) as plain strings, so
      `codepack-core` needs no dependency on `codepack-security`
- [+] Fingerprint is stable and documented: never contains the secret, and is derived
      only from values that are already safe to write down (I3)
- [+] Applied in `scan` and `verify` only — **not** inside `codepack-security`, not in
      the engine pipeline, not in the ~30 reports. Rationale to record: changing the
      domain crate's output would move golden parity and touch I5's artifact contracts
      for a convenience feature. Boundary stated in the module doc, not left implicit
- [+] Suppressed findings are **counted and reported as suppressed**, never silently
      dropped — a scanner that quietly hides findings is worse than one that is noisy
- [+] `scan --json` gains a `fingerprint` per finding so a user can copy it into the
      file without hand-deriving it (additive field, no schema bump per `output.rs`)
- [+] Tests: fingerprint stability; a matching entry suppresses and is counted; a
      non-matching entry does not; unknown key in the file is an error naming the key;
      missing file is simply "no allowlist". **Rule decided explicitly:** the exit code
      is computed from what survives, so suppressing every `critical` finding does
      return 0 — that is the point of a reviewed allowlist, and it is also why a
      malformed fingerprint is rejected at load rather than left matching nothing.
      Additionally rejected: an uppercase-hex fingerprint, so one finding has exactly
      one spelling.

## A3 — `codepack scan --staged`

- [+] `ScanArgs` gains `--staged`
- [+] Lists staged entries via `git2` (index vs `HEAD`), added/modified only, never
      deleted paths
- [+] **Scans the staged blob content, not the working-tree file.** A pre-commit hook
      must answer "what is about to be committed"; if a file is staged and then edited,
      those differ. Blobs are materialised into a temp dir and scanned there; temp dir
      cleaned up on every exit path. Recorded as a deliberate correctness choice
- [+] Not a git repository → clear error (exit 1), not a silent empty result
- [+] No staged files → exit 0 with an explicit "nothing staged" message, not a
      confusing empty report
- [+] `--staged` composes with `--json` and with the A2 allowlist
- [+] Tests: repository built programmatically with `git2` (no `git` binary, per the
      domain rule); staged clean file → 0; staged `.env` → 3; secret in the working
      tree but **not** staged → 0 (proves it reads the index, the whole point);
      staged-then-edited file → the *staged* content is what gets scanned; not-a-repo
      and nothing-staged paths
- [+] **Fixed after review:** `Repository::discover` walks upward, so in a monorepo the
      diff returned every staged path in the whole repository while the report still
      named the project the user asked about. The diff is now scoped with a pathspec to
      the project being scanned; two tests pin it (a sibling package's staged file is
      excluded; scanning the repository root still sees everything).
- [+] **Recorded, not silently widened (Q25):** the frozen exit-code contract gates on
      `critical` only, so a staged `.env` stops a commit but `export API_KEY=…` in a
      staged script does not. Covered by an honest test that asserts the `high` finding
      is reported *and* that it does not gate, rather than by quietly changing a
      contract other people's pipelines depend on.

## Verification (run at the very end, all three together)

- [+] `cargo fmt --all --check`
- [+] `cargo clippy --workspace --all-targets -- -D warnings`
- [+] `cargo test --workspace`
- [+] `cargo xtask gate` (full: fmt, clippy, tests, `cargo deny check`, frontend
      format/typecheck/lint, `scripts/` suite, `sync-agents --check`, network isolation)
- [+] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment
- [+] Independent review pass (`codepack-quality-reviewer`) before merge. Found two
      real defects and one dishonest test, all fixed before merge: (1) `verify`
      reported ~24 findings for a provably clean bundle, because the checklist item
      about the bundle's own reports had been skipped rather than resolved; (2)
      `scan --staged` scanned the whole repository instead of the given project;
      (3) the clean-bundle test asserted only `critical == 0`, so it passed while (1)
      was true. Also raised and accepted as-is: the `high`/`critical` gating gap
      (Q25), the allowlist's scope boundary (Q26), and the absence of a size cap on
      untrusted archive extraction (noted below).
- [-] Bounded extraction for `verify`: **not implemented.** A hostile archive can fill
      the temp volume before anything notices. Low severity for a local CLI opening a
      file the user chose, and no acceptance criterion covers it, but it is a real
      gap in a command explicitly aimed at archives from third parties. Left as an
      honest note rather than a silent omission.

## Completion

- [+] `docs/architecture/overview.md` — `codepack-cli` row updated (`verify`,
      `scan --staged`), `codepack-core` row updated (`allowlist`)
- [+] `docs/decisions/open-questions.md` — three new questions recorded: **Q25**
      (`--staged` gates on `critical` only), **Q26** (the allowlist's scope stops at
      `scan`/`verify`), **Q27** (`verify`'s content/generated split and its residual
      short-password limit, shared with Q18)
- [+] Checklist filled `+`/`-` honestly, final report in Russian
- [+] Push to `origin`, fast-forward merge into `main`, delete every branch but `main`
      (local and remote) — only after the full gate is green

---

## Next task

Not yet chosen. Start with the orientation ritual from
`.ai/project/13-progress-tracking.md`.
