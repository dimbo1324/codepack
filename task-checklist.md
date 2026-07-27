# Task Checklist

**Task:** Stage S13 — direct AI integration. Close the loop "export → answer" without
manual copy-paste, both over an API and by handing the bundle to a local coding agent.
**Date:** 2026-07-27
**Branch:** feat/s13-ai-integration

## Owner decisions taken before starting (recorded in the decisions log)

- **Both shapes.** API integration (BLUEPRINT §B.8) **and** a no-network handoff to a
  local agent (Claude Code / Codex), which are different problems: an agent already
  reads the filesystem, so shipping it a bundle over HTTP would be absurd.
- **Anthropic first, trait for the rest.** Closes Q2 partially: one working provider
  plus the abstraction, so a second arrives as a module rather than a rewrite.
- **One question, one answer, saved into the bundle** — no chat UI in this stage.
- **Summary and confirmation before sending** — counts and size, not a per-file list.

## What bounds this task

- [ ] **Invariant I1.** `codepack-ai` is the only crate permitted a network client.
      Enforced by a gate check, not by convention — the same reasoning that made the
      desktop enforce filesystem isolation with capabilities instead of a rule.
- [ ] **Stage order.** S13 comes after S12, which is `готов (частично)`; Q19/Q20 stay
      open and are not touched here.
- [ ] **No key ever reaches a file, a log, history, or an export** (the stage's own
      "готово, когда" criterion).

## Implementation

### `crates/codepack-ai` (new crate)

- [ ] `provider.rs` — the `AiProvider` trait plus request/response types; nothing in it
      names a vendor
- [ ] `providers/anthropic.rs` — Messages API over raw HTTP (`ureq`: blocking, rustls,
      no async runtime — `reqwest` would drag tokio into a tree that has none)
- [ ] `keys.rs` — API key stored only in the OS credential store (`keyring`), never in
      `Config`
- [ ] `plan.rs` — the `SendPlan` the UI shows for confirmation: provider, model, files,
      bytes, estimated tokens, redaction state, critical-finding count
- [ ] **Refuse to send a bundle carrying critical findings** unless explicitly
      overridden; this product exists to stop that leak, so the default cannot be "send"
- [ ] `handoff.rs` — the offline path: write a ready-to-use prompt beside the extracted
      bundle and hand back the command to run, with no network and no key

### Wiring

- [ ] `Config` gains AI fields (`#[serde(default)]`, no `schema_version` bump — additive)
- [ ] Desktop commands, all thin over the crate; key never crosses back to the frontend
- [ ] Frontend page: pick prompt, review the plan, confirm, read the answer
- [ ] i18n keys in both `en.ts` and `ru.ts`

### Guard

- [ ] A gate step proving no crate other than `codepack-ai` declares an HTTP client

## Verification

- [ ] Unit tests for the trait, the plan, the refusal rule, key handling
- [ ] A test asserting the key appears in no artifact, log, or history row
- [ ] `cargo xtask gate` green
- [ ] Real end-to-end call against the Anthropic API **only if** the owner supplies a
      key; otherwise the network path is tested against a stub and that is stated
      plainly rather than implied

## Completion

- [ ] `ROADMAP.md` S13 `**Status.**` line (Russian), §1 table updated
- [ ] `docs/architecture/overview.md`, `invariants.md` (I1 now has an enforced exception)
- [ ] `docs/decisions/open-questions.md`: Q2 resolution, the HTTP-client choice, the
      critical-findings refusal
- [ ] Checklist filled `+`/`-`, final report in Russian
