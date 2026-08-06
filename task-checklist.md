# Task Checklist

**Task:** Four owner-approved capabilities in one branch — the S13 door for local agents,
git-history scanning, stable redaction pseudonyms, and a ready-made pre-commit hook plus
a GitHub Action.

**Date:** 2026-08-06
**Branch:** feat/agent-handoff-history-scan-pseudonyms-ci-hooks

Owner instruction, 2026-08-06: implement proposals 1, 3, 5 and 6 in one pass, test
everything, then merge into `main` and push.

## Preparation

- [ ] Orientation ritual: git status/log with `--date=iso-strict`, ROADMAP, overview,
      previous checklist, open questions
- [ ] Read the four affected areas before touching them: `codepack-ai`,
      `codepack-security` redaction, `codepack-cli` (`scan`, `staged`), desktop commands
      and the frontend client
- [ ] Commit this checklist before the work

## Implementation — 1. The S13 door (local agent handoff)

- [ ] `codepack-ai` gains an `api` feature so the **offline** handoff can be depended on
      without the HTTP client and the credential store coming with it (invariant I1: a
      front end that only prepares a handoff must not carry a network client at all)
- [ ] `Config`: `ai_handoff_agent` + `ai_handoff_question`, normalized, `.codepack.toml`
      keys, project-config round trip
- [ ] CLI: `codepack handoff <bundle>` — `--agent`, `--question`, `--json`
- [ ] Desktop: `list_local_agents` and `prepare_handoff` commands over the same crate
- [ ] Frontend: handoff card on the Result page, typed client functions, RU/EN strings

## Implementation — 2. Scanning git history

- [ ] `codepack-cli::history_scan` — walk commits through `git2`, deduplicate by blob id,
      materialise into a temporary directory, never shell out to `git`
- [ ] `scan --history`, `--since <ref>`, `--max-commits <n>`; mutually exclusive with
      `--staged`
- [ ] Findings carry the commit they came from, at full timestamp precision
      (`.ai/universal/09-time-and-timestamps.md`)

## Implementation — 3. Stable redaction pseudonyms

- [ ] `codepack-security::Redactor` — plain (today's behaviour, unchanged) or labelled
- [ ] Labels are assigned by first-seen order, never derived from the secret's value:
      a hash would be a testable commitment to the secret and would weaken I3
- [ ] `Config::redaction_labels`, default `false`, so every existing configuration and
      every golden reference produces byte-identical output
- [ ] Wired into the two surfaces an assistant actually reads: `03_text_dump.txt` and the
      git reports

## Implementation — 4. Pre-commit hook and GitHub Action

- [ ] `codepack init --hook` — installs the hook into someone else's project, honouring
      `core.hooksPath`, refusing to overwrite a foreign hook without `--force`
- [ ] `scan --sarif <file>` — the Action needs SARIF from `scan`, not only from `export`
- [ ] `scan --fail-on <severity>`, default `critical` so the published exit-code contract
      is unchanged (closes Q25 by option (a))
- [ ] `action.yml` at the repository root + README documentation

## Verification

- [ ] New tests for every item above, including the negative cases
- [ ] `cargo xtask fmt`
- [ ] `cargo xtask gate` — every section green
- [ ] Real runs, not only tests: `handoff`, `scan --history`, `scan --sarif`,
      `init --hook` against a scratch repository
- [ ] Frontend `typecheck` and `lint`
- [ ] Self-review of the diff

## Documentation

- [ ] `README.md` — commands, hook, Action, pseudonyms
- [ ] `docs/architecture/overview.md` — what changed shape
- [ ] `docs/__arch__/ROADMAP.md` — status lines
- [ ] `docs/__arch__/open-questions.md` — the owner decisions this task acts on

## Completion

- [ ] Checklist filled with `+`/`-`
- [ ] Merge into `main` fast-forward, push to `origin/main`
- [ ] Final report
