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

- [ ] Golden compares `06_security_scan.json` but **not** `03_text_dump.txt`. So the
      redaction fix has no golden impact; the detector fix does.
- [ ] The scanner emits **at most one hit per line**, chosen by a documented cascade.
      New rules must sit late in that cascade, or an already-detected line changes its
      rule/severity and golden breaks for no gain.
- [ ] `find_secret_spans` feeds **both** redaction and the scanner's keyword confidence.
      Widening it in place would promote provider hits to `secret_like_line`/`high` —
      the exact regression the cascade's doc comment warns about. Redaction needs its
      own wider span set instead.
- [ ] Invariant I9: precision stays 1.000. Recall may only go up.

## Implementation

- [ ] `find_url_credentials` — the password between `://user:` and `@host`, matched by
      **structure** not entropy, so a short password is caught as reliably as a long one
- [ ] `Basic`/`Digest` auth alongside the existing `Bearer` matcher, without disturbing
      the legacy-parity `find_bearer_tokens` used by the scanner's keyword path
- [ ] Scanner: both new rules added **after** the weak-keyword step, before entropy
- [ ] Redaction: its own span set — keyword roots widened to the scanned set, plus
      provider, telegram, URL credentials and HTTP auth
- [ ] Entropy deliberately **excluded** from content redaction, with the reason written
      down

## Verification

- [ ] A test that fails before the fix: an exported bundle's text dump must not contain
      a planted AWS key
- [ ] Corpus gains positives (URL credential, Basic auth) and negatives (a URL with no
      credentials, `http://host/a:b@c`)
- [ ] `cargo test -p codepack-engine --test golden` still 3/3 — proving the cascade
      placement was right rather than assuming it
- [ ] Precision still 1.000, recall higher
- [ ] End-to-end rerun of the audit's own reproduction
- [ ] `cargo xtask gate` green

## Completion

- [ ] `docs/decisions/open-questions.md`: the decision and what it costs; Q18 closed
- [ ] Checklist filled `+`/`-`, final report in Russian
- [ ] Push to origin (explicitly requested)
