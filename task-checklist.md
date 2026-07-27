# Task Checklist

**Task:** Close audit findings 1 and 2 — a critical secret reaching the text dump
unredacted, and two classes of secret the detector cannot see.
**Date:** 2026-07-27
**Branch:** fix/secret-redaction-and-recall

Owner instruction: fix both, then push. The instruction is also the owner decision the
audit said was needed for any divergence from legacy behaviour.

## What the audit established, and what changes because of it

- **Finding 1.** `redact_secrets` uses only legacy's keyword shapes, while the scanner
  gained provider signatures and entropy in S3. A key the scanner calls `critical` is
  written verbatim into `03_text_dump.txt`. The code names the gap itself:
  `SCAN_ONLY_ROOTS` is documented as "scanned for but never redacted".
- **Finding 2.** No rule sees a password inside `scheme://user:pass@host`, and `Basic`
  auth is uncovered while `Bearer` is.

## Constraints discovered before writing code (these shape the fix)

- [+] Golden compares `06_security_scan.json` but **not** `03_text_dump.txt`. So the
      redaction fix has no golden impact; the detector fix does.
- [+] The scanner emits **at most one hit per line**, chosen by a documented cascade.
      New rules must sit late in that cascade, or an already-detected line changes its
      rule/severity and golden breaks for no gain.
- [+] `find_secret_spans` feeds **both** redaction and the scanner's keyword confidence.
      Widening it in place would promote provider hits to `secret_like_line`/`high` —
      the exact regression the cascade's doc comment warns about. Redaction needs its
      own wider span set instead.
- [+] Invariant I9: precision stays 1.000. Recall may only go up.

## Implementation

- [+] `find_url_credentials` — the password between `://user:` and `@host`, matched by
      **structure** not entropy, so a short password is caught as reliably as a long one
- [+] `Basic`/`Digest` auth alongside the existing `Bearer` matcher, without disturbing
      the legacy-parity `find_bearer_tokens` used by the scanner's keyword path
- [+] Scanner: both new rules added **after** the weak-keyword step, before entropy
- [+] Redaction: its own span set — keyword roots widened to the scanned set, plus
      provider, telegram, URL credentials and HTTP auth
- [+] Entropy deliberately **excluded** from content redaction, with the reason written
      down

## Verification

- [+] A test that fails before the fix: an exported bundle's text dump must not contain
      a planted AWS key — `codepack-engine/tests/pipeline.rs::planted_secrets_never_reach_the_text_dump`,
      reproducing all four of the audit's own planted secrets end to end
- [+] Corpus gains positives (URL credential, Basic auth) and negatives (a URL with no
      credentials, `http://host/a:b@c`, plus a bare username, a plain URL, and
      `Basic`/`digest` used as ordinary words)
- [+] `cargo test -p codepack-engine --test golden` still 3/3 — proving the cascade
      placement was right rather than assuming it
- [+] Precision still 1.000, recall higher (`cargo test -p codepack-security --test corpus`)
- [+] End-to-end rerun of the audit's own reproduction — all four planted secrets
      (`hunter2fakepass`, `AKIAIOSFODNN7EXAMPLE`, the Basic auth base64 blob, the API key)
      now found and none reach any `Finding.message` or `03_text_dump.txt`
- [+] `cargo xtask gate` green (quick gate: fmt/clippy/deny/frontend/agents-sync/network
      isolation) plus a full, separate `cargo test --workspace` (1001 tests, 0 failed)

## Completion

- [+] `docs/decisions/open-questions.md`: the decision and what it costs; Q18 closed
- [+] `docs/architecture/overview.md`: `codepack-security` row updated (new
      `patterns/credentials.rs`, widened `redact.rs`, two new scanner rules, 183 tests)
- [+] Checklist filled `+`/`-`, final report in Russian
- [+] Push to origin (explicitly requested)

## Honest notes on scope

- **`Digest` support is narrow by design, and the code says so.** A real `Digest`
  header is a comma-separated list of short quoted fields
  (`username="...", realm="...", response="..."`), so no single run after the keyword
  clears the 16-character floor this heuristic shares with `Bearer`. What it actually
  catches is the less common case of an unquoted value (e.g. a bare `response=<hex>`
  hash) logged by a client or proxy. Both the typical case (not caught) and the
  atypical case (caught) are pinned by tests, not left implicit.
- **The redaction placeholder for a `Basic`/provider match containing an internal `=`
  or `:` is not pretty** (`Basic <REDACTED>=<REDACTED>` rather than a single clean
  token) — this is pre-existing `replace_match`/`sanitize_key_prefix` behaviour applied
  to a new span type, not a new defect. It never leaks (verified by test); it is just
  not cosmetically minimal. Left as-is rather than special-cased, since correctness is
  what invariant I3 requires and the existing shared implementation is what closed the
  Q16 divergence in the first place.
- Findings 3–8 in the audit (mutex poisoning, an unwired settings toggle, an unused
  npm dependency, three over-limit files, the `AGENTS.md` budget, the unwired
  `codepack-ai` crate) are **out of scope for this task** — the owner asked specifically
  for findings 1 and 2. Not silently dropped: they remain in `AUDIT-2026-07-27.md` for
  a future task.
