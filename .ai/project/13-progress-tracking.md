# Project Progress Tracking

Purpose: any assistant, in any session, on any model, can locate exactly where the
project stands and where it is going — **from files, not from memory**. This is the
primary recovery mechanism after a lost conversation.

## Where the truth lives

| Question | File |
|---|---|
| What the product is: logic, formats, math | `BLUEPRINT.md` |
| What is planned, in what order, what is done | `ROADMAP.md` |
| What is actually built right now | `docs/architecture/overview.md` |
| What must never break | `docs/architecture/invariants.md` |
| Owner decisions and open questions | `docs/decisions/open-questions.md` |
| What the current or last task was | `task-checklist.md` |
| What actually happened recently | `git log --oneline -15` |
| How the rules themselves changed | `.ai/CHANGELOG.md` |
| How the legacy version worked | `docs/__arch__/codepack-main.zip` |

## Orientation ritual — at the start of EVERY task

In order, without skipping:

1. `git status --short --branch` and `git log --oneline -15`.
2. `ROADMAP.md` §1 and the `**Status.**` lines under each stage: a stage with a status
   line is done; **the first stage without one is next**.
3. `docs/architecture/overview.md` — what exists in the code right now.
4. `task-checklist.md` — what the previous task was and whether it finished cleanly.
5. `docs/decisions/open-questions.md` — whether a decision changes the plan.
6. Only then plan the new task.

If the task touches behavior that existed in the legacy version, also consult the legacy
reference module.

## Update duties when finishing work

- Completed a stage or a significant slice → add or refresh the `**Status.**` line under
  that stage in `ROADMAP.md` (what shipped: crates, modules, commands, tests) and update
  the status column in §1. Write it in Russian to match that file.
- Changed the system's shape (new crate, new layer, new operational job) → update
  `docs/architecture/overview.md`.
- Made or received an owner decision that constrains the future → record it in
  `docs/decisions/open-questions.md`, not only in the chat.
- Introduced an invariant → record it in `docs/architecture/invariants.md`.
- Changed a rule module → record it in `.ai/CHANGELOG.md` and regenerate `AGENTS.md`.

## Drift guard

If the plan, the state document, and the code disagree: **the code is the fact, the plan
is the intent**. Reconcile them in the same task or report the mismatch explicitly.
Stale documentation is worse than no documentation.

## Unfinished-task rule

If `task-checklist.md` still holds open `[ ]` items from a previous session, resolve
them first: finish them, or mark them `-` with an honest note. Starting a new task on
top of a silently abandoned one is a violation.
