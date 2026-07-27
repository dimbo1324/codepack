# Task Checklist

**Task:** Stage S13 — direct AI integration. Close the loop "export → answer" without
manual copy-paste, both over an API and by handing the bundle to a local coding agent.
**Date:** 2026-07-27
**Branch:** feat/s13-ai-integration

## Owner decisions taken before starting (recorded in the decisions log)

- [+] **Both shapes.** API integration (BLUEPRINT §B.8) **and** a no-network handoff
- [+] **Anthropic first, trait for the rest** — partially closes Q2
- [+] **One question, one answer, saved into the bundle** — no chat UI
- [+] **Summary and confirmation before sending** — counts and size, not a file list

## What bounds this task

- [+] **Invariant I1.** `codepack-ai` is the only crate permitted a network client,
      enforced by a gate step reading every manifest — not by convention
- [+] **Stage order.** S12's open questions (Q19/Q20) untouched
- [+] **No key in a file, log, history, or export** — the key lives only in the OS
      credential store, and `check` runs before it is ever read

## Implementation

### `crates/codepack-ai` (new crate)

- [+] `provider.rs` — `AiProvider` trait; nothing in it names a vendor
- [+] `providers/anthropic.rs` — Messages API over raw HTTP
- [+] `keys.rs` — key only in the OS credential store, never in `Config`
- [+] `plan.rs` — `SendPlan` for confirmation: provider, model, files, bytes, tokens,
      scan state
- [+] **Refuses to send a bundle carrying critical findings** unless explicitly overridden
- [+] `handoff.rs` — the offline path: prompt file beside the bundle, command to run

### Wiring

- [-] **`Config` AI fields — NOT DONE**
- [-] **Desktop commands — NOT DONE**
- [-] **Frontend page and i18n — NOT DONE**

The domain layer is complete and tested; the door to it from the application is not cut.
The feature is therefore **not reachable by a user**. This is the honest state, not a
rounding error: I ran out of room to do the interface properly and chose to leave it
undone rather than ship a half-wired page.

### Guard

- [+] `cargo xtask gate` step proving no crate but `codepack-ai` declares an HTTP client,
      with hermetic tests that fail when a violation is planted

## Verification

- [+] 32 tests in `codepack-ai`, 12 in `xtask` (was 9); workspace 938 → 982
- [+] The refusal rule, the ordering (`check` before key read), and the `None` vs
      `Some(0)` distinction each have their own test
- [+] `cargo xtask gate` green end to end
- [-] **No real API call was made.** No key was supplied, so the network path is verified
      by parsing real response shapes, not by a live request. Those are different things
      and the second is not claimed.
- [-] Independent review of the diff — not run

## Completion

- [+] `ROADMAP.md` S13 `**Status.**` line and §1 row, both saying "partially"
- [+] `docs/architecture/invariants.md` — I1 now documents how it is enforced
- [+] `docs/decisions/open-questions.md` — the two-shapes decision, Q2 partial
      resolution, the critical-findings refusal, the TLS/licence decision
- [-] `docs/architecture/overview.md` — not updated (the new crate is not in it yet)
- [+] Checklist filled `+`/`-`, final report in Russian

## What the next task must pick up

1. `Config` fields (`ai_enabled`, `ai_provider`, `ai_model`), additive with
   `#[serde(default)]` — no `schema_version` bump needed
2. Tauri commands, thin over the crate; the key must never cross back to the frontend —
   `keys::has_key` exists precisely so status can be rendered without reading the secret
3. The frontend page and its i18n keys in both dictionaries
4. `docs/architecture/overview.md`
5. One real API call against a key the owner supplies, to close the last verification gap
