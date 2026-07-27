# Task Checklist

**Task:** Close audit findings 2–7 (`AUDIT-2026-07-27.md`), then merge to `main`, push,
and delete every branch but `main`.
**Date:** 2026-07-27
**Branch:** fix/secret-redaction-and-recall

Finding 2 was completed and pushed in the previous task on this branch; findings 3–7
follow here. Finding 8 is deliberately out of scope (see below).

## Findings

- [+] **2 — detector blind to connection-string passwords and Basic auth** (done
      previously on this branch): `patterns/credentials.rs`, structural matching on
      *position* in the URL rather than shape, closing Q18. Corpus gained 2 positives and
      5 negatives; precision stayed 1.000, recall rose; golden stayed 3/3.
- [+] **3 — `.expect("never poisoned")` on nine mutex locks.** Replaced with one `lock()`
      helper using `unwrap_or_else(|p| p.into_inner())`. The old message asserted an
      invariant nobody had proved; the new comment argues why ignoring poison is right
      *for this data* (a live-run table and one watcher handle: a foreign panic cannot
      make either wrong, only incomplete). Recovery is **proved by test** — a thread
      panics holding the lock and the registry must keep working.
- [+] **4 — `watch_clipboard_auto_update` shown but wired to nothing.** Implemented
      rather than removed: the backend deliberately delegates to the frontend, and
      `copyText` already exists needing no permission. Summary formatting lives in its
      own module (`lib/util/watchSummary.ts`) so it is testable without mounting a
      component. A failed copy is **reported**, not swallowed.
- [+] **5 — unused `@tauri-apps/plugin-clipboard-manager`.** Removed from `package.json`
      and the lockfile. The comment in `clipboard.ts` that justified not using it by
      saying it "is already a dependency" was corrected — it would otherwise have become
      false.
- [+] **6 — three files over the project's own ~600-line limit.** All split by the
      precedent already set for `commands/export.rs`, public surface unchanged:
      `scan/mod.rs` 890→487, `token_scan.rs` 825→505 (directory module),
      `orchestrator.rs` 688→526. The orchestrator needed more than a test extraction
      (611 lines of logic), so two self-contained concerns moved out whole:
      `staging.rs` (the RAII cleanup guard plus the tests that were already only about
      it) and `cancelled.rs`.
- [+] **7 — `AGENTS.md` at 30.0 of 30 KiB.** Two paragraphs in `11-commands.md`
      duplicated `15-command-reference.md` **verbatim**; removed. Formatting and
      pre-commit-hook detail moved to the same extended-tier module. Now 29.0 KiB. The
      limit was not raised: Codex reads at most 32 KiB, so exceeding it hides rules
      silently instead of failing.

## Verification

- [+] `cargo xtask gate --quick` green (fmt, clippy, deny, frontend, agents sync,
      network isolation)
- [+] `cargo test --workspace` — 1003 passed, 0 failed
- [+] `cargo test -p codepack-engine --test golden` — 3/3, parity unmoved
- [+] New tests prove the fixes rather than assert them: two poisoning-recovery tests
      (finding 3), and the split modules' own suites still pass unchanged (finding 6)
- [+] `svelte-check` 135 files, 0 errors (finding 4's new module and handler)
- [+] `AGENTS.md` regenerated and in sync at 29.0 KiB (finding 7)

## Completion

- [+] `docs/decisions/open-questions.md`: one record covering findings 3–7, Q22 updated
      with the new headroom and the rule for the next overflow
- [+] Checklist filled `+`/`-`, final report in Russian
- [+] Merge to `main`, push, delete every other branch locally and on the remote

## Out of scope, and why

- **Finding 8 (`codepack-ai` unwired)** is stage S13 work with an honest status already
  recorded in `ROADMAP.md`; the audit itself calls it "known and documented", not a
  defect. Not touched.
- **The dependency-checker (`depcheck`/`knip`) suggested inside finding 5** would be a
  new dependency and a new gate step — a separate decision, not a side effect of
  deleting one unused package. Not added.
- Findings 3 and 4 were verified by test and typecheck, **not** by launching the desktop
  app and watching files change by hand. The clipboard write itself (a DOM API inside a
  real webview) is therefore covered by reasoning and types, not by an observed
  end-to-end run.
