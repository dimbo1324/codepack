# Task Checklist

**Task:** Audit and harden the developer script orchestrator (`dev_tools_scripts_runner.py`
and `scripts/`): fix what is broken, make the failures actionable, close the security and
robustness gaps.
**Date:** 2026-07-27
**Branch:** fix/dev-scripts-hardening

Owner report: "некоторые скрипты не работают, или работают коряво", plus an explicit ask
to cover the security angles. Every item below was reproduced before it was written down
— no speculative fixes.

## Defects found, with the evidence

- [+] **D1 — every script crashes on a legacy console code page.** Reproduced: cp866
      crash on the em dash, ascii crash, cp1251 fine. Cyrillic was never the cause.
- [+] **D2 — `clean-project` refuses with no remedy.** Reproduced end to end by unsetting
      `core.longpaths` and re-running against the real pnpm tree.
- [+] **D3 — `doctor` gives a false all-clear on git hooks.** Reproduced: `core.hooksPath`
      unset in this clone, doctor green.
- [+] **D4 — Windows paths mangled in the interactive prompt.** Reproduced:
      `--out C:\Users\dev\build` → `['--out', 'C:Usersdevbuild']`.
- [+] **D5 — `EOFError` uncaught.** Reproduced with a stream that claims to be a tty.
- [+] **D6 — no subprocess timeout.**
- [+] **D7 — undecodable paths cannot be distinguished** from real ones.
- [+] **D8 — `remove_all` never proved containment.**
- [+] **D9 — `run_steps` hid the steps it skipped.**
- [+] **D10 — dead code in safety-relevant paths** (unreachable shim suffix, no-op
      `except OSError: raise`, mutable-default `type: ignore`, deprecated `onerror`).
- [+] **D11 — `prune_empty_dirs` descended into `.git`/`node_modules`/`target`.**

## Implementation

- [+] **Encoding** — `_toolkit/terminal.py`: a degrading error handler on stdout/stderr,
      installed from `scripts/__init__.py` so it precedes any print. Terminals keep their
      encoding and degrade `—`→`--`; redirected streams switch to UTF-8.
- [+] **doctor** — checks `core.hooksPath` and, on Windows, `core.longpaths`; both are
      warnings naming the exact command. Version probes now time out.
- [+] **clean-project** — recognises the known git warnings and prints the cure; refuses
      on undecodable paths; proves containment before deleting; `onexc`.
- [+] **processes** — timeouts, `capture_bytes`, `NOT_FOUND`/`TIMED_OUT`, dead branch gone.
- [+] **interactive/execution** — EOF leaves cleanly at all three prompts and at the
      confirm prompt; `split_arguments` keeps Windows backslashes.
- [+] **steps** — skipped steps are named "not run (earlier step failed)".
- [+] Tests: 40 → 78, wired into `selftest` and therefore the gate.

## Verification

- [+] Each fix carries a test; the `prune_empty_dirs` one was checked against a
      deliberately re-broken copy and does fail without the fix
- [+] `selftest` green (78 tests)
- [+] Every catalog script launched: doctor, selftest, clean-project, format-code --check,
      install-hooks, help. `doctor` → `install-hooks` → `doctor` closes the loop
- [+] `cargo xtask gate` green end to end
- [-] `dev-run` and `build-installer` **not** executed. Both do a full release build of
      the desktop app; neither was touched by this task beyond the shared toolkit, which
      the other six exercise. Not run, not claimed.
- [+] Self-review of the diff, which caught two defects I had introduced (see below)

## Defects found in my own diff during review

- [+] The 30 s capture timeout also applied to `git status` in the deletion planner. On a
      kernel-sized tree with antivirus that is ordinary, so a healthy slow run would have
      become a refusal. Given its own 600 s bound with the reasoning written down.
- [+] Pruning `dirs` before recording them made a directory whose only child is a skipped
      one look empty, so a dry run would have promised a removal an apply cannot perform.
      The unpruned list is now what the emptiness test reads.
- [+] A stale comment left claiming `topdown=False` after the walk became top-down.

## Completion

- [+] `.ai/project/11-commands.md` updated (two factual corrections), `.ai/CHANGELOG.md`
      entry added, `AGENTS.md` regenerated
- [+] `docs/decisions/open-questions.md`: the output-degradation decision and the
      fail-closed decision on undecodable paths
- [+] Checklist filled `+`/`-`, final report in Russian

## Debt this task leaves

- `AGENTS.md` is at **30.0 KiB of its 30 KiB budget**. My addition to `11-commands.md` had
  to be cut twice to fit. The next module edit will not fit at all; a module needs
  tightening or moving to the extended tier before then.
- `dev-run` and `build-installer` have no tests and were not run here.
- The Windows-junction gap in `discovery` is unchanged: `is_symlink()` reports False for
  junctions, so they are traversed. That errs toward protecting more, not less.
