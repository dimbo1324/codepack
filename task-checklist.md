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

**What that costs, named before the work started:** `ureq` and `keyring` return to the
product, to `Cargo.lock` and to `cargo deny`'s graph; `keyring` is compiled on every
platform again; the Linux CI and packaging legs need `libsecret-1-dev`. This reverses the
build-level benefit of Q41 while keeping its code-level one (the transport still lives in
exactly one crate, and the gate still refuses it to every other).

> **The `libsecret-1-dev` half of that prediction was wrong, and checking beat guessing.**
> `keyring` 4.2's Linux backend is `zbus-secret-service-keyring-store` — pure-Rust D-Bus,
> not the libsecret C library — so no header package is needed, and `libdbus-1-dev` was
> already installed. Read out of the crate's own lockfile rather than assumed.

## ⚠️ The constraint that dominates this task's honesty

**This machine has no Rust toolchain.** No `cargo`, no `rustup`, no `pnpm`, no `gh`, and
no `target/` — this working tree has never been built here. So **`cargo xtask gate` was
never run**: not the build, not the tests, not clippy, not `cargo deny`, not rustfmt, and
not the `Cargo.lock` regeneration that `ureq` and `keyring` now require.

Everything checkable without a toolchain was checked, and is marked as such below.
Everything that needs one is marked `-`. See the final report for what has to happen next.

## Step 0 — preparation

- [+] Orientation ritual: git state, ROADMAP, overview, previous checklist, open questions
- [+] Previous checklist confirmed closed (2.0.1 polish, every item marked)
- [+] Branch off up-to-date `main` (`git pull --ff-only` confirmed already current)
- [+] This checklist committed **before** the work started (commit `92466f5`)

## Step 1 — the crate returns to the workspace

- [+] `workspace.exclude` entry removed from the root `Cargo.toml`
- [+] `codepack-ai-api` inherits `version`/`edition`/`rust-version`/`lints` from the
      workspace; the comment explaining the duplication went with the duplication
- [+] `ureq` and `keyring` declared in `[workspace.dependencies]`, both decision comments
      moved there verbatim rather than dropped
- [+] The crate's own `Cargo.lock` deleted — a member resolves through the workspace's
- [-] `cargo deny check` passes with both back in the graph. **Not run**: no toolchain.
      This is the check most likely to have something to say, since it is the one that
      judges licences and advisories on `ureq`'s and `keyring`'s transitive trees
- [-] Workspace `Cargo.lock` regenerated. **Not done**: needs cargo. Dozens of packages
      with checksums cannot be hand-written honestly, and merging the excluded crate's old
      lockfile by hand would be a guess wearing a checksum's clothes

## Step 2 — the gate learns the one exception

- [+] `network_isolation` permits `codepack-ai-api` to declare a client, and only it
- [+] Second rule added, which is the stricter half: only `codepack-cli` and
      `codepack-desktop` may depend on that crate, so no transport can sit under the
      export pipeline where no user action gates it
- [+] Tests rewritten around the new rule: a front end may depend, a domain crate may not,
      the exception may declare a client, and any *other* crate declaring one still fails
- [+] Found and fixed a test that was passing for the wrong reason:
      `every_listed_client_is_detected` wrote no root manifest, so `check` failed during
      member discovery before reading a single dependency — the assertion would have held
      for an empty denylist
- [-] The real workspace passes its own check. **Not run**: it is a Rust test

## Step 3 — retire what existed only because of the exclusion

- [+] `cargo xtask ai-api` removed — usage line, dispatch arm, module, and the separate
      `format (ai-api)` sections in both `gate` and `fmt`
- [+] The version-drift test went with it: `version.workspace = true` makes drift
      inexpressible rather than merely unlikely
- [+] `.github/workflows/ai-api-weekly.yml` deleted — the gate covers the crate now
- [+] `clean-project`'s sweep of `crates/codepack-ai-api/target/` and its note removed;
      that directory only existed because the crate built outside the workspace
- [+] `.ai/project/10-project-map.md`, `12-domain-rules.md`, `15-command-reference.md`
      updated; `.ai/CHANGELOG.md` entry written
- [+] `AGENTS.md` regenerated — see the note below on how, since `cargo xtask sync-agents`
      could not run

## Step 4 — configuration

- [+] Four fields: `ai_api_enabled` (false), `ai_api_provider`, `ai_api_model` (empty means
      "the most capable this build knows"), `ai_api_question`
- [+] `normalized_ai_api_provider` falls back like every other string field; the model is
      deliberately *not* a set, so a model released after this build still works (Q2)
- [+] The key is **not** a field and cannot be: settings are a file people export and share
- [+] The JSON contract test lists all four new keys, so the count assertion still holds
- [+] A drift guard in `codepack-ai-api` asserts its provider list and core's
      `AI_API_PROVIDERS` name the same providers — the same shape `codepack-ai` already
      uses for the local-agent list
- [-] Those tests actually run. **Not run**: no toolchain

## Step 5 — the command

- [+] `codepack ask <bundle>`: plan, guard, send, answer appended to `AI_ANSWER.md`
- [+] `codepack key set|status|clear`, mirroring the `settings` subcommand shape
- [+] **The key is read from stdin, never from a flag** — `ps` and shell history are
      neither of them somewhere this program can clean up after itself
- [+] Terminal echo is *not* suppressed and the prompt says so, rather than implying
      otherwise: hiding it needs `unsafe` FFI (forbidden workspace-wide) or a crate for
      one prompt. The masked field is on the desktop screen, which is the front end for
      typing
- [+] No `key show` command, and no report struct has a field a key could occupy — a test
      pins the serialized shape of each
- [+] `--json` carries `schema_version` and the `command` discriminator; machine output
      stays on stdout alone
- [+] Exit codes: a critical-findings refusal is **3** (the code that already means
      "worked, found critical secrets"), a switched-off integration or transport failure
      is **1**. Unit tests assert the mapping
- [+] `--dry-run` prints the plan, works with the integration off, and reads no key
- [+] Completions and the man page pick the new commands up automatically — both generate
      from the clap `Cli`, so there was no list to forget to update
- [+] `open_bundle` extracted to `commands::bundle` with its tests, so `handoff` and `ask`
      cannot drift about what counts as a bundle
- [+] Six integration tests added against the real binary: dry run, the default refusal,
      exit 3 on critical findings, `null`-not-`0` for an unscanned bundle, an unknown
      provider, and `key status` never carrying a key
- [-] Those tests actually run. **Not run**: they spawn the compiled binary

## Step 6 — the screen

- [+] Five Tauri commands: status, plan, ask, store-key, clear-key
- [+] The key crosses the IPC boundary inbound only; the screen renders from a
      `key_stored` boolean that `has_key` answers without reading the secret, and a test
      pins the serialized shape so a field cannot be added for convenience
- [+] The key input is cleared the moment the key is stored — a value left in a bound
      input stays in the webview's memory and reaches a screenshot
- [+] The send runs on a background thread and emits one run-id-filtered `ai:finished`
      event, mirroring `sanitize`. The window does not block
- [+] The bundle goes through `ValidatedResultPath` like every other bundle command
- [+] Settings section: switch, model, masked key field, default question, a warning
- [+] Ask card on the result page, with the plan shown before anything leaves
- [+] `None` is rendered "not verified", never as a reassuring zero — on both front ends
- [+] Critical findings refuse; the override is its own checkbox that must be ticked
      before the send button enables
- [+] EN and RU strings for everything, verified by script: 413 keys each, no drift, no
      duplicates, and every key used on either page exists
- [+] The `handoff__*` CSS block, now serving both S13 cards, renamed `panel__*` so the
      names stop lying
- [-] `pnpm typecheck`/`lint`/`format` and the Rust tests. **Not run**: no pnpm, no cargo

## Step 7 — CI and packaging

- [+] No system package needed after all — see the note at the top. The CI comment that
      explained *keeping* `libdbus-1-dev` in July now records that it was the right call
- [-] `install-and-run-linux` still installs and runs on all three distributions.
      **Not verified**: needs a CI run

## Step 8 — verification

- [-] `cargo xtask gate` fully green locally. **Not run at all** — no toolchain
- [+] What could be checked without one, was:
      - the `scripts/` suite, which is Python and is part of the full gate: **78 tests
        pass**, so removing the `clean.json` entry broke nothing
      - `clean.json` still parses
      - `AGENTS.md` regeneration, via a Python transcription of `xtask::sync_agents`
        that was **first proved to reproduce the committed file byte for byte** from
        unchanged modules — that proof is why its output for the edited modules is
        trustworthy. It also caught the budget: 30.0 KiB with **32 bytes** of headroom
      - i18n parity and every translation key used on the two pages
      - Svelte `{#if}`/`{#each}` and `<section>` balance on both edited pages
      - the provider's model list is non-empty and most-capable-first, which is what the
        model fallback and its test depend on
- [-] The new command exercised against a real exported bundle. **Not done**: needs a build
- [-] **No live request to a provider**, exactly as the plan said. It needs a real API key,
      which this session does not have and must not handle. The network leg remains proven
      by response parsing and tests, not by an exchange — the same honest gap S13 has
      carried since 2026-07-27

## Step 9 — completion

- [+] `docs/__arch__/ROADMAP.md`: S13's `**Status.**` says "сделано" for the first time,
      with the missing live request named in the same line; §1 status column updated
- [+] `docs/architecture/invariants.md`: I1 rewritten — the restored exception, both rules
      that now enforce it, and why the second one is the stricter half
- [+] `docs/architecture/overview.md`: crate row, network-isolation row, both front-end
      sections, fourteen commands, the weekly workflow row deleted, and the known-debt list
      updated — one item closed, two opened (no live request, no cancellation)
- [+] `docs/__arch__/open-questions.md`: the owner decision with all three options and why
      two were declined; Q41 amended rather than contradicted
- [+] `README.md`: an "Ask a model directly" section, the command table, and the privacy
      guarantee restated — it claimed no crate here reaches the network, which is no
      longer true as written
- [+] `CHANGELOG.md`: an Unreleased entry written for someone using codepack
- [+] Checklist filled with `+`/`-`
- [+] Final report

## What this task did not do

- **It was never compiled.** This is the headline, not a footnote. The code is written and
  statically reviewed; it has not been type-checked by a compiler, and a change this size
  across Rust, Tauri commands and Svelte almost certainly has something for the compiler
  to say. `Cargo.lock` is also not regenerated, and `cargo deny`'s verdict on `ureq`'s and
  `keyring`'s trees is unknown
- **No live provider request.** By design and by necessity
- **No cancellation of a send.** `ureq` gives no handle to interrupt a request in flight,
  so neither front end offers one, rather than offering a button that lies
- **No version bump and no release.** Not asked for; the branch is not merged and nothing
  was pushed
- **`AGENTS.md` is 32 bytes under its hard limit.** Q22 has warned since July. The next
  module edit has to shrink something or mark a module `tier: extended` first — this task
  had to trim its own two sentences twice to fit
