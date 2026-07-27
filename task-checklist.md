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

- [ ] **D1 — every script crashes on a legacy console code page.** `console.heading` and
      the menus print `—`; on cp866 (the default for Russian `cmd.exe`) the process dies
      with `UnicodeEncodeError` before doing any work. Reproduced: cp866 crash, ascii
      crash, cp1251 fine.
- [ ] **D2 — `clean-project` refuses with no remedy.** Any `git status` stderr is a
      fail-closed refusal (correct), but the message does not name the cause. The owner's
      actual case was `core.longpaths` unset plus deep pnpm paths.
- [ ] **D3 — `doctor` gives a false all-clear on git hooks.** It checks that the tracked
      file `.githooks/pre-commit` exists — always true — instead of checking
      `core.hooksPath`. Reproduced: hooks inactive in this clone, doctor green.
- [ ] **D4 — Windows paths are mangled in the interactive prompt.** `shlex.split` in posix
      mode eats backslashes: `--out C:\Users\dev\build` → `['--out', 'C:Usersdevbuild']`.
- [ ] **D5 — `EOFError` is uncaught.** Ctrl+Z/Ctrl+D at the menu gives a traceback.
- [ ] **D6 — no subprocess has a timeout.** A hung tool (a Windows Store `python` stub is
      the classic) blocks `doctor` forever.
- [ ] **D7 — undecodable paths cannot be distinguished.** The deletion planner decodes git
      output with `errors="replace"`, then matches protection rules against the mangled
      name.
- [ ] **D8 — `remove_all` never proves the target stays under the repository root.**
- [ ] **D9 — `run_steps` hides the steps it skipped** after a required failure; the
      summary reads as though they did not exist.
- [ ] **D10 — dead code in safety-relevant paths:** the unreachable `""` shim suffix, the
      no-op `except OSError: raise`, `failures: list = None` behind a `type: ignore`, and
      `shutil.rmtree(onerror=)` which is deprecated since 3.12.
- [ ] **D11 — `prune_empty_dirs` descends into `.git`, `node_modules` and `target`**
      instead of pruning them from the walk, then discards the result.

## Implementation

- [ ] **Encoding** — one place that makes stdout/stderr UTF-8 safe on Windows, with an
      ASCII-degrading fallback so output is never a crash. Applies to every script.
- [ ] **doctor** — check what is actually true: `core.hooksPath` points at `.githooks`,
      `core.longpaths` on Windows, and keep the checks config-driven.
- [ ] **clean-project** — recognise the known git warnings and print the exact remedy;
      refuse on undecodable paths; assert containment before deleting; `onexc`.
- [ ] **processes** — timeouts, dead branch removed.
- [ ] **interactive/execution** — handle EOF, split arguments correctly on Windows.
- [ ] **steps** — report skipped steps honestly.
- [ ] Tests for every fix above, wired into `selftest` and therefore the gate.

## Verification

- [ ] Each defect reproduced by a test that fails before the fix
- [ ] `python dev_tools_scripts_runner.py selftest` green
- [ ] Every script in the catalog launched and its behaviour checked by hand
- [ ] `cargo xtask gate` green
- [ ] Independent review of the diff

## Completion

- [ ] `.ai/project/11-commands.md` updated if the workflow changed
- [ ] `docs/decisions/open-questions.md` for anything that constrains the future
- [ ] Checklist filled `+`/`-`, final report in Russian
