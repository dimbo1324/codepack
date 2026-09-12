# Task Checklist

**Task:** Finish stage S13's API path — give it a command and a screen. The domain layer
(`codepack-ai-api`: `ask`, the key store, the plan and its guards, the Anthropic client)
has been complete and tested since 2026-07-27 and reachable by nobody. This task makes it
reachable from both front ends.

**Date:** 2026-09-12
**Branch:** `feat/s13-api-path-command-and-screen`

Owner instruction, 2026-09-12: «добей S13 — API-путь получает команду и экран».

## The decision this task rests on

Finishing S13 requires a workspace member to depend on `codepack-ai-api`, which the
`network isolation` gate step refuses by design and which invariant I1 forbids outright
since 2026-09-06 (Q41). That is an invariant change, and the rules put it to the owner.

**Owner decision, 2026-09-12: the API path ships in the release build.** The crate returns
to the workspace, both front ends depend on it, and I1 goes back to its original wording —
one named exception, only on an explicit user action. Two options were declined: a
cargo feature off by default (S13 would be finished for nobody who downloads the
installer) and a separate `codepack-ask` binary (no screen, and process-spawning from the
webview breaks the two-front-ends-over-one-engine shape).

**What that costs, named before the work starts:** `ureq` and `keyring` return to the
product, to `Cargo.lock` and to `cargo deny`'s graph; `keyring` is compiled on every
platform again; the Linux CI and packaging legs need `libsecret-1-dev`. This reverses the
build-level benefit of Q41 while keeping its code-level one (the transport still lives in
exactly one crate, and the gate still refuses it to every other).

## Step 0 — preparation

- [ ] Orientation ritual: git state, ROADMAP, overview, previous checklist, open questions
- [ ] Previous checklist confirmed closed (2.0.1 polish, every item marked)
- [ ] Branch off up-to-date `main`
- [ ] This checklist committed **before** the work starts

## Step 1 — the crate returns to the workspace

- [ ] `workspace.exclude` entry removed from the root `Cargo.toml`
- [ ] `codepack-ai-api` inherits `version`/`edition`/`rust-version`/`lints` from the
      workspace instead of spelling them out, and the comment explaining the duplication
      goes with the duplication
- [ ] `ureq` and `keyring` declared in `[workspace.dependencies]`, decision comments moved
      there rather than dropped
- [ ] `cargo deny check` passes with both back in the graph — licences and advisories

## Step 2 — the gate learns the one exception

- [ ] `network_isolation` permits `codepack-ai-api` to declare a client, and **only** it
- [ ] A member that depends on `codepack-ai-api` is now permitted where it was refused —
      but the denylist still refuses a client declared anywhere else
- [ ] Tests rewritten around the new rule, including the negative: any other crate taking
      `ureq` still fails
- [ ] The real workspace passes its own check

## Step 3 — retire what existed only because of the exclusion

- [ ] `cargo xtask ai-api` — the crate is gated like every other member now
- [ ] The version-drift test in xtask's `ai_api` module — inheritance makes drift impossible
- [ ] `.github/workflows/ai-api-weekly.yml` — the gate covers the crate on every push
- [ ] `.ai/project/11-commands.md` and `15-command-reference.md` updated, `AGENTS.md`
      regenerated, `.ai/CHANGELOG.md` entry

## Step 4 — configuration

- [ ] `Config` gains the API-path fields (enabled, provider, model, question) with
      defaults that keep the feature off until the user turns it on
- [ ] Normalization: an unknown provider or model falls back rather than failing
- [ ] The key is **not** among them and cannot be: it lives only in the OS store
- [ ] Round-trip and normalization tests, as every other field has

## Step 5 — the command

- [ ] `codepack ask <bundle>` — plan, guard, send, save the answer
- [ ] `codepack key set|status|clear` for the credential store, mirroring `settings`
- [ ] **The key is read from stdin, never from a flag** — a flag lands in shell history
      and in `ps` output
- [ ] `--json` carries `schema_version` and the `command` discriminator, machine output on
      stdout only
- [ ] Exit codes honour the published contract; a refusal is distinguishable from a failure
- [ ] `--dry-run` prints the plan without sending, so the guard can be seen working
- [ ] Completions and man page regenerate with the new commands

## Step 6 — the screen

- [ ] Tauri commands for status, plan, ask, store-key, clear-key
- [ ] The key crosses the IPC boundary only inbound; nothing ever returns it
- [ ] The send runs on a background thread and the window does not block
- [ ] Key, provider, model and the enable toggle on the settings page
- [ ] An ask card on the result page, next to the existing local-agent handoff card
- [ ] The plan is shown before sending: files, bytes, estimated tokens, critical findings
      as "not verified" when the scan did not run
- [ ] A critical finding refuses, and the override is a separate explicit action
- [ ] EN and RU strings for everything added

## Step 7 — CI and packaging

- [ ] `libsecret-1-dev` added to the Linux gate leg and the packaging workflows
- [ ] `install-and-run-linux` still installs and runs on all three distributions

## Step 8 — verification

- [ ] `cargo xtask gate` fully green locally
- [ ] The new command exercised against a real exported bundle, up to the network boundary
- [ ] **No live request to a provider.** It needs the owner's API key, which this session
      does not have and must not handle. The send path is proven by tests and by response
      parsing, not by an exchange — the same honest limit S13 has carried since 2026-07-27

## Step 9 — completion

- [ ] `docs/__arch__/ROADMAP.md`: S13 `**Status.**` refreshed, §1 status column updated
- [ ] `docs/architecture/invariants.md`: I1 rewritten with the restored exception and how
      it is enforced now
- [ ] `docs/architecture/overview.md`: the crate's row, the two-front-end section, and the
      known-debt entries that this closes
- [ ] `docs/__arch__/open-questions.md`: the owner decision, Q41 amended, Q2 revisited
- [ ] `README.md`: the new commands, and what turning the feature on means
- [ ] `CHANGELOG.md`: an entry for users
- [ ] Checklist filled with `+`/`-`
- [ ] Final report
