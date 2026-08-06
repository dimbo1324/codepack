# Task Checklist

**Task:** An MCP server as the third entry point — `codepack mcp`, so a coding agent can
ask `preview`, `scan`, `explain` and `export` itself instead of being handed a bundle.

**Date:** 2026-08-06
**Branch:** feat/mcp-server

Owner instruction, 2026-08-06: implement proposal 2 (the MCP server), run every test
again, then push to `main`.

## Preparation

- [+] Read what the CLI commands already build, so the tools answer exactly what the
      commands answer rather than becoming a second implementation
- [+] Commit this checklist before the work

## Decisions made honestly, before code

- [+] **Where it lives.** The original proposal said "a thin crate over
      `codepack-engine`". Reading the code said otherwise: `preview`, `scan` and
      `explain` are not engine calls — they are the CLI's four-layer config resolution,
      its forced `full` safe mode for scanning, its budget handling and its report
      shapes. It went into `codepack-cli` as a directory module instead, and the
      estimate is corrected out loud in the module doc, the ROADMAP and the decisions log
- [+] **stdio only.** JSON-RPC 2.0 over newline-delimited stdin/stdout. No new
      dependency, so invariant I1 and the gate's network-isolation step are untouched
- [+] **stdout carries protocol and nothing else.** The export tool runs quiet; progress
      still goes to stderr. The end-to-end test fails on any non-JSON line on stdout, so
      this is checked rather than asserted in a comment

## Implementation

- [+] `mcp/protocol.rs` — JSON-RPC envelopes, error codes, the one-line framing
- [+] `mcp/tools.rs` — four tools with JSON schemas and annotations, dispatching onto the
      existing command builders
- [+] `mcp/mod.rs` — the read/dispatch/write loop, `initialize`/`ping`/`tools/list`/
      `tools/call`
- [+] `commands::export` split into `run` (printing, exit code) and `build` (the work),
      the shape `preview`/`scan`/`explain` already had
- [+] `codepack mcp` subcommand
- [+] A tool failure is `isError: true` inside a successful result; JSON-RPC errors are
      reserved for protocol faults
- [+] The preview file list is capped at 400 and reports `files_truncated`/`files_total`

## Verification

- [+] 152 unit tests in `codepack-cli` (up from 121): protocol shapes, notifications
      answered with silence, malformed input mid-session, unknown method, unknown tool,
      unknown safe mode, each tool's dispatch, the cap
- [+] 76 end-to-end tests (up from 70): six of them spawn the real binary and speak the
      protocol over pipes — handshake, `tools/list`, `explain` on a `.env`, `scan`,
      a real `export`, a tool failure, a malformed message mid-session
- [+] `cargo xtask fmt`
- [+] `cargo xtask gate` — all eight sections green
- [+] A real session driven by hand: an agent asking why `.env` is missing gets
      `verdict=excluded, reason=high-risk credential filename`
- [+] Whole suite re-run under a runner-style short `TEMP` — 56/56, the condition that
      broke CI on the previous task
- [+] Self-review of the diff: 12 files, no stray files, no new dependency

## Documentation

- [+] `README.md` — a section with the `claude mcp add` line, the JSON config and the
      four tools; the commands table and "what you get"
- [+] `docs/architecture/overview.md` — the third surface, why it is not a third front
      end, and the no-cancellation limit under known debt
- [+] `docs/__arch__/ROADMAP.md` — an S13 addendum, including the corrected estimate
- [+] `docs/__arch__/open-questions.md` — the decision record, Q37 and Q38

## Completion

- [+] Checklist filled with `+`/`-`
- [+] Merge into `main` fast-forward, push to `origin/main`
- [+] Final report

## Debt carried out of this task

- Q37: one request at a time, no cancellation — a long export cannot be interrupted from
  the client, even though the engine supports it.
- Q38: the protocol version is a constant; a new MCP revision needs a code change.
- No `resources` or `prompts` capability, and no handoff tool (an agent handing a bundle
  to itself is a circle).
