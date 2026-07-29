---
name: code-review
description: Use to review the current diff before finalizing a task — scope, architecture boundaries, security, invariants, test honesty, and documentation drift.
---

# Reviewing Changes

Run this **before** merging and before the final report.

## What to look at

```powershell
git status --short --branch
git diff
```

For an independent pass, delegate to the `codepack-quality-reviewer` subagent.

## Mandatory checklist

1. **Scope.** No changes the task did not require: incidental refactoring, renames,
   reformatting unrelated files, redesign, removed functionality.
2. **Architecture.** Dependencies point downward (`engine` → domain → `core`); the core
   does not know about the UI; no cycles; business logic has not leaked into the
   frontend; modules stay under roughly 600 lines.
3. **Security.** No secrets in code, tests, or fixtures. Findings are redacted before
   reaching a log, report, history entry, or database row. No network calls outside
   stage S13. Path-traversal checks and symlink non-following intact.
4. **Invariants** (`docs/architecture/invariants.md`): source immutability, privacy,
   preserved byte reporting, artifact format compatibility.
5. **Tests.** Not deleted, disabled, or weakened. Secret-detector accuracy thresholds
   not lowered. New behavior covered.
6. **Cancellation.** Long loops check the cancellation token inside, not only between
   steps.
7. **Documents.** Shape changed → `docs/architecture/overview.md` updated; stage
   completed → `**Status.**` line in `docs/__arch__/ROADMAP.md`; rule module changed →
   `.ai/CHANGELOG.md` updated and `AGENTS.md` regenerated.
8. **Leftovers.** Debug remnants, temp files, commented-out code, comments restating a
   function name.

## Handling findings

Serious findings — security, data, invariants — are fixed in the same task. Unrelated
problems are recorded separately rather than mixed into the current diff. If there are
no findings, say so plainly instead of inventing remarks.
