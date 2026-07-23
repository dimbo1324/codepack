# Task Checklist

**Task:** Stage **S3 — Security (`codepack-security`)** (`ROADMAP.md` §2). ⭐ core
value of the whole project.
**Date:** 2026-07-23
**Branch:** feat/s3-security-policy-redact-scanner

Scope boundary (binding for this task): `codepack-security` depends only on
`codepack-core` (ROADMAP §1 dependency table: S3 → S1). No production dependency on
`codepack-scanner`. No wiring into `ExportPlan.severity` (S9's job). No tree-walking in
production code — the scan API takes a caller-supplied file list; `walkdir` is a
dev-dependency only, for building this crate's own test fixtures. No SQLite persistence
(S5), no clipboard/text-dump call sites (S9), no network validation ever (I1, permanent).

## Preparation

- [+] Orientation ritual: git status/log, ROADMAP.md §1 (S3 first stage without
      `**Status.**`), docs/architecture/overview.md, task-checklist.md,
      docs/decisions/open-questions.md — no open items blocking S3
- [+] Delegated stage planning to `codepack-stage-planner`: legacy archive extracted to
      a scratchpad temp dir and read directly (`constants.py`, `services/export_policy.py`,
      `utils/text_utils.py`, `reports/insights/security.py`, `services/risk_preview.py`,
      matching legacy test files as behavioral oracles)
- [+] Resolved a real porting blocker ahead of coding: legacy's keyword/value redaction
      regex uses a backreference (`(\2)` requiring the closing quote to match the
      opening quote) — Rust's `regex` crate has no backreference support. The plan
      supplies a behavior-preserving, backreference-free rewrite (three alternatives:
      double-quoted / single-quoted / unquoted, each still forbidding embedded
      whitespace) to implement, with a dedicated match-span-equivalence test.
- [+] Resolved scope tension: `codepack-security` must depend only on `codepack-core`
      per ROADMAP's own dependency table (S3 → S1, not S2) — this forces duplicating
      `TEXT_EXTENSIONS`/`BINARY_EXTENSIONS`/`TEXT_FILENAMES_WITHOUT_EXTENSION`/
      `should_consider_text_file`/`looks_binary` from `codepack-scanner` rather than
      sharing them. Recorded as tech debt (hoist candidate for `codepack-core`, likely
      at S9 when both crates are first combined) rather than doing an unrequested
      refactor of S2's merged code inside this task.
- [+] Resolved scope gap: legacy tries 6 encodings when reading file content
      (utf-8, utf-8-sig, cp1251, cp866, utf-16, latin-1); S3 reads UTF-8 (+ lossy
      fallback) only — the full chain's real owner is S9's text-dump step, pulling in
      an encoding-detection dependency for S3 alone would be scope creep. Recorded
      honestly, not silently dropped.
- [+] Resolved: no legacy precision/recall/F1 baseline exists anywhere in the archive —
      S3 establishes the first baseline itself; the corpus test's measured numbers
      become what invariant I9 subsequently protects.

## Implementation — parity first

- [+] `Cargo.toml`: `aho-corasick` in `[workspace.dependencies]`; `codepack-core`/
      `serde`/`serde_json`/`thiserror`/`regex`/`aho-corasick` wired into
      `crates/codepack-security/Cargo.toml`; `walkdir`/`tempfile` as dev-dependencies
      only — verified with a clean `cargo build -p codepack-security`
- [+] `error.rs` — `SecurityError` (thiserror) + `Result<T>`
- [+] `constants.rs` — `SENSITIVE_FILENAMES`, `SENSITIVE_SUFFIXES`,
      `HIGH_RISK_FILENAMES`, `SAFE_MODE_EXCLUDED_SUFFIXES`,
      `BALANCED_MODE_EXCLUDED_SUFFIXES` — diffed byte-for-byte against the legacy
      archive by two independent passes (implementation + review), not eyeballed
- [+] `classify.rs` — duplicated `should_consider_text_file`/`looks_binary` +
      `TEXT_EXTENSIONS`/`BINARY_EXTENSIONS`/`TEXT_FILENAMES_WITHOUT_EXTENSION`
      (documented duplication, see tech-debt note above, Q7 in open-questions.md)
- [+] `policy/` — `SafetyDecision`, `normalise_mode`, `is_env_example`,
      `classify_sensitive_file`, `should_skip_file_for_safety` (exact precedence:
      full never skips; balanced checks `.env`/`HIGH_RISK_FILENAMES` then
      `BALANCED_MODE_EXCLUDED_SUFFIXES`; safe delegates to `classify_sensitive_file`),
      `SecurityOptions` + `From<&codepack_core::config::Config>`
- [+] `redact.rs` — `redact_secrets`, including the backreference-free rewrite,
      documented inline as a deliberate behavior-preserving syntax change
- [+] `patterns/keyword.rs` — `SECRET_PATTERNS`-equivalent, `SECRET_KEY_PATTERN`,
      `ASSIGNMENT_SECRET_RE`, `PRIVATE_KEY_RE`, `secret_confidence` (5-level cascade:
      self-protection exemption → critical PEM → high SECRET_PATTERNS → medium
      assignment-shaped → low bare-keyword-outside-comment → none), `redacted_line`
      (redacts the whole line, never just the matched span), self-protection hint list
- [+] `patterns/risky_code.rs` — the 9 risky-code rules verbatim (including the
      intentional python-eval/js-eval regex duplication — not deduplicated),
      confidence fixed at `"medium"` for all risky-code findings regardless of rule
      severity (legacy quirk, ported not "fixed")
- [+] `scan/` — `Finding`/`FindingKind`/`ScanResult`, `scan_project()` (caller-supplied
      file list, no walking), `.txt`/`.json`/`.sarif` writers matching legacy's exact
      structure (SARIF: minimal-but-valid 2.1.0, `level` = error for critical/high,
      warning otherwise, never "note" — the omission is ported too)
- [+] `lib.rs` — crate doc stating the S3 scope boundary explicitly, public
      re-exports, under 100 lines

## Implementation — then new (🎯)

- [+] `patterns/provider.rs` — 10 provider-signature rules (AWS, GitHub, Google,
      Slack, Stripe, OpenAI, Anthropic, Telegram, JWT, PEM-reuse); Anthropic's
      `sk-ant-` pattern is checked/ordered so it is never shadowed by the OpenAI `sk-`
      pattern (`anthropic_key_wins_over_openai_pattern` test); each authored-not-ported
      regex flagged as such in a comment
- [+] `patterns/entropy.rs` — Shannon entropy calculator; base64-like (H≥4.0 bits/char,
      len≥20) and hex-like (H≥3.0, len≥32) thresholds; alphabet-conformance
      tokenization (split on non-alphabet chars, score each token); context boost
      (adjacent `=`/`:`/secret-shaped identifier raises confidence one bucket;
      bare high-entropy token with no context caps at `low`, never discarded)
- [+] `patterns/prefilter.rs` — `aho-corasick` literal prefilter (keyword roots +
      provider literal prefixes + `-----BEGIN` + `eyJ`) gating the regex/entropy pass;
      superset-safety regression test; entropy-only findings (no keyword/provider
      literal) documented as the prefilter's known scope limit — entropy scanning
      runs unconditionally, never gated by the prefilter
- [+] Wired provider/entropy findings into `scan::scan_project` at the documented
      confidence levels

## Verification

- [+] Unit tests colocated per module (policy precedence table incl. `.env`/
      `.env.example`/`.env.sample` and `HIGH_RISK_FILENAMES` vs `SENSITIVE_FILENAMES`
      distinction, `redact_secrets` match-span equivalence to the backreference
      original, each of the 9 risky-code rules, self-protection hint list, entropy
      thresholds + context boost). 111 colocated unit tests.
- [+] Golden fixture test (`tests/golden_fixture.rs`): end-to-end scan reproduces the
      legacy finding set (sensitive files + all 4 confidence levels + all 9
      risky-code rules + self-protection) with matching types/severities/confidence/
      messages
- [+] SARIF structural validity against the 2.1.0 core-required fields
      (`tests/sarif_validity.rs`, 4 tests)
- [+] Corpus test (`tests/corpus.rs`): 17 synthetic labeled positives / 14 negatives
      (never real leaked credentials), legacy-parity-mode vs full-mode comparison.
      **Measured baseline (new I9 baseline):** parity mode TP=5 FP=0 FN=11 →
      precision=1.000, recall=0.312, F1=0.476; full mode TP=16 FP=0 FN=0 →
      precision=1.000, recall=1.000, F1=1.000. `recall(full) > recall(parity)` and
      `precision(full) ≥ precision(parity)` both hold.
- [+] I3 audit: `tests/i3_no_secret_leak.rs` plants one secret per detector family and
      asserts none leak into serialized JSON output or any `Finding.message`. A real
      I3 violation was found and fixed during the independent review pass (below) —
      `mask_non_keyword_secret_spans` dropped an overlapping-but-not-contained span
      instead of extending the redaction over it, letting the tail of a secret leak
      when a short fixed-length provider match and a longer entropy-tokenized span
      shared a start offset. Regression test added:
      `scan::tests::overlapping_provider_and_entropy_spans_mask_the_full_extent`.
- [+] Confirmed zero network-capable dependency in the resolved dependency tree (I1) —
      `cargo tree -p codepack-security`: only aho-corasick/memchr, the regex family,
      codepack-core's serde/serde_json/thiserror/crossbeam-channel, dev-deps
      tempfile/walkdir.
- [+] `cargo xtask gate` green, verified in the main working directory (fmt, clippy
      `-D warnings`, tests, `cargo deny check`, `sync-agents --check`)
- [+] No `unsafe`; no bare `unwrap()`/`expect()` outside tests without a
      proven-invariant comment

## Completion

- [+] `docs/architecture/overview.md` updated
- [+] `ROADMAP.md`: `**Status.**` line under S3 + §1 table status updated, honestly
      listing every deviation (backreference rewrite, encoding-chain scope gap,
      constant duplication + hoist candidate, provider-regex authorship vs literal
      port, corpus baseline numbers, no `schema_version` bump)
- [+] Independent review pass (`codepack-quality-reviewer`) before merge — found and
      the calling session fixed: (1) CRITICAL — the I3 overlapping-span leak above;
      (2) HIGH — `scan_project` took no `CancellationToken` at all, contradicting the
      domain rule that long operations must check cancellation inside their loop
      (fixed: `cancel: &CancellationToken` parameter, checked once per file, matching
      `codepack-scanner::walk_project`'s per-entry granularity from S2); (3) MEDIUM —
      this checklist/ROADMAP/overview were still showing S3 as not started despite the
      code existing (fixed by this very completion pass); (4) LOW — inconsistent
      `expect()` justification comments across `patterns/risky_code.rs` and
      `provider.rs` vs `keyword.rs` (fixed, unified phrasing). Nothing was found that
      required weakening a threshold or test.
- [-] CI green on all three OSes — **not verified this task.** The branch has not been
      pushed and CI (`.github/workflows/ci.yml`) only triggers on `push: main` or a
      pull request, neither of which has happened yet. Gate is green locally; CI
      confirmation is deferred to when the owner decides how this branch reaches
      `main` (push a WIP branch for a PR check, or ff-merge locally first).
- [+] Commits: checklist first (already on branch from the planning commit), then
      implementation, separated logically from this completion pass
- [-] Fast-forward merge into `main` — **not done.** Per workflow, merging requires the
      full gate green (it is) and this is reversible/local, but pushing the result to
      `origin/main` needs explicit owner sign-off, not yet obtained in this
      conversation.
- [+] Final report to owner (Russian, per language policy)

---

## Next task

Stage **S4 — Diff и снапшоты (`codepack-diff`)** (`ROADMAP.md` §2). Start with the
orientation ritual from `.ai/project/13-progress-tracking.md`.
