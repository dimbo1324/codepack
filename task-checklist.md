# Task Checklist

**Task:** Four owner-approved capabilities in one branch — the S13 door for local agents,
git-history scanning, stable redaction pseudonyms, and a ready-made pre-commit hook plus
a GitHub Action.

**Date:** 2026-08-06
**Branch:** feat/agent-handoff-history-scan-pseudonyms-ci-hooks

Owner instruction, 2026-08-06: implement proposals 1, 3, 5 and 6 in one pass, test
everything, then merge into `main` and push.

## Preparation

- [+] Orientation ritual: git status/log with `--date=iso-strict`, ROADMAP, overview,
      previous checklist, open questions
- [+] Read the four affected areas before touching them: `codepack-ai`,
      `codepack-security` redaction, `codepack-cli` (`scan`, `staged`), desktop commands
      and the frontend client
- [+] Commit this checklist before the work

## Implementation — 1. The S13 door (local agent handoff)

- [+] `codepack-ai` gains an `api` feature so the **offline** handoff can be depended on
      without the HTTP client and the credential store coming with it. Verified with
      `cargo tree`: neither front end links `ureq` or `keyring`
- [+] `Config`: `ai_handoff_agent` + `ai_handoff_question`, normalized, agent ids in
      `valid_sets` (dependency points `ai → core`), a test in `codepack-ai` keeps the two
      lists from drifting. `.codepack.toml` deliberately does **not** carry them: they are
      per-user, not a team decision
- [+] CLI: `codepack handoff <bundle>` — `--agent`, `--question`, `--json`; a `.zip` is
      unpacked beside itself so the agent can read it after the process exits
- [+] Desktop: `list_local_agents` and `prepare_handoff` over the same crate
- [+] Frontend: handoff card on the Result page, typed client functions, RU/EN strings

## Implementation — 2. Scanning git history

- [+] `codepack-cli::history_scan` — `git2` revwalk, first-parent diffs, deduplication by
      blob id, materialised into a temporary directory; no `git` binary anywhere
- [+] `scan --history`, `--since <ref>`, `--max-commits <n>`; `--staged` conflicts with it
      at the argument level, so one report can never claim to be both
- [+] Findings are relabelled onto repository paths before the allowlist runs, and each
      names its commit with a full `YYYY-MM-DD HH:MM:SS UTC` stamp
- [+] Both limits (500 commits, 8 MiB per file version) are reported; truncation prints as
      a `WARNING`, not as a statistic

## Implementation — 3. Stable redaction pseudonyms

- [+] `codepack-security::Redactor` — plain or labelled; `Placeholders` threaded through
      the redaction functions rather than read from a global
- [+] Labels assigned by first-seen order. A hash of the value was rejected and the reason
      recorded: it is a checkable commitment to the secret, which weakens I3
- [+] `Config::redaction_labels`, default `false`; a test asserts plain mode is byte for
      byte what the free functions produced, which is why no golden reference moved and no
      `schema_version` changed
- [+] Wired into `03_text_dump.txt` and the git reports; the header says what `<REDACTED:sN>`
      means. `verify` now takes the placeholder list from `codepack-security` instead of
      keeping its own copy
- [-] **Not** wired into the ~30 insight reports or scan findings — that would move
      `06_security_scan.json` and SARIF (I5). Recorded as open question Q34

## Implementation — 4. Pre-commit hook and GitHub Action

- [+] `codepack init --hook` — honours `core.hooksPath`, updates its own hook, refuses a
      foreign one without `--force`
- [+] `scan --sarif <file>`, written from the findings that survived the allowlist
- [+] `scan --fail-on <severity>`, default `critical` — closes Q25 by option (a) with the
      published exit-code contract unchanged
- [+] `action.yml` at the repository root + README documentation
- [-] The hook does **not** block a commit when `codepack` is missing; it warns loudly
      instead. The trade-off is deliberate and recorded as Q35

## Verification

- [+] New tests: 56 suites green, up from 913 to 1004 assertions' worth of coverage across
      `codepack-security` (+6), `codepack-cli` (+27 unit and end-to-end), `codepack-engine`
      (+2 pipeline), `codepack-desktop` (+4), `xtask` (+3)
- [+] `cargo xtask fmt`
- [+] `cargo xtask gate` — all eight sections green: format, clippy `-D warnings`, tests,
      cargo-deny, frontend format/typecheck/lint, dev scripts (78 tests), agents sync,
      network isolation
- [+] Real runs against a scratch repository, not only tests: a credential committed and
      deleted is invisible to `scan` and found by `scan --history` with its commit;
      `init --hook` blocked a real `git commit` carrying a staged `.env`; `--fail-on high`
      turned a `high` finding into exit 3 while the default stayed 0; `--sarif` produced
      valid 2.1.0; a labelled export gave the shared secret `s1` in two files and `s2` to
      the other, with no raw value anywhere; `handoff` unpacked a real bundle and printed
      the command; `verify` stayed clean on that labelled bundle
- [+] `cargo tree` on both front ends: no `ureq`, no `keyring`
- [+] Frontend `typecheck` (137 files, 0 errors) and `lint`
- [+] Self-review of the diff: 40 files changed, no stray files, no debug leftovers
- [-] No browser verification of the Result page's handoff card: it renders Tauri command
      output, so a bare Vite server would show an empty agent list and prove nothing.
      Covered by `svelte-check`, ESLint and the backend command tests instead

## Documentation

- [+] `README.md` — four new sections (handoff, history scanning, labelled redaction, CI),
      the commands table, and the pre-commit section rewritten around `init --hook`
- [+] `docs/architecture/overview.md` — the `api` feature split, the eleven CLI commands,
      the Action, and three new entries under known debt
- [+] `docs/__arch__/ROADMAP.md` — S13's status line updated, S14 gains the 2026-08-06
      addendum describing all three out-of-stage capabilities
- [+] `docs/__arch__/open-questions.md` — the decision record for this task, Q25 closed,
      Q34/Q35/Q36 opened

## Completion

- [+] Checklist filled with `+`/`-`
- [+] Merge into `main` fast-forward, push to `origin/main`
- [+] Final report

## Debt carried out of this task

- Q34: labels stop at the text dump and the git reports, so one bundle spells the same
  secret two ways depending on which artifact you read.
- Q35: the hook lets a commit through on a machine without codepack.
- Q36: `--history` defaults to 500 commits rather than the whole history.
- The GitHub Action builds from source on every run (no signed release binaries — S14),
  which costs minutes of runner time per job.
