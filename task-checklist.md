# Task Checklist

**Task:** An MCP server as the third entry point — `codepack mcp`, so a coding agent can
ask `preview`, `scan`, `explain` and `export` itself instead of being handed a bundle.

**Date:** 2026-08-06
**Branch:** feat/mcp-server

Owner instruction, 2026-08-06: implement proposal 2 (the MCP server), run every test
again, then push to `main`.

## Preparation

- [ ] Read what the CLI commands already build, so the tools answer exactly what the
      commands answer rather than becoming a second implementation
- [ ] Commit this checklist before the work

## Decisions to make honestly, before code

- [ ] **Where it lives.** The original proposal said "a thin crate over
      `codepack-engine`". Reading the code says otherwise: `preview`, `scan` and
      `explain` are not engine calls — they are the CLI's four-layer config resolution,
      its forced `full` safe mode for scanning, its budget handling and its report
      shapes. A separate crate would have to restate all of that, and the two would
      drift. It goes in `codepack-cli` as a directory module instead, and the estimate
      is corrected out loud rather than quietly
- [ ] **stdio only.** JSON-RPC 2.0 over newline-delimited stdin/stdout. No HTTP client,
      no new dependency, so invariant I1 and the gate's network-isolation step are
      untouched
- [ ] **stdout carries protocol and nothing else.** Same rule `--json` already lives by;
      progress and diagnostics go to stderr, or the first log line breaks the session

## Implementation

- [ ] `mcp/protocol.rs` — JSON-RPC envelopes, error codes, the newline framing
- [ ] `mcp/tools.rs` — the tool catalogue with JSON schemas, and dispatch onto the
      existing command builders
- [ ] `mcp/mod.rs` — the read/dispatch/write loop
- [ ] `commands::export` — extract a `build` from `run`, the way `preview`/`scan`/
      `explain` already have one, so the tool and the command share it
- [ ] `codepack mcp` subcommand
- [ ] A tool failure is `isError: true` inside a successful result, not a JSON-RPC
      error: the agent has to be able to read and act on it
- [ ] Cap the preview file list and say when it was capped

## Verification

- [ ] Unit tests: protocol shapes, unknown method, notifications answered with silence,
      malformed input, each tool's dispatch
- [ ] End-to-end: spawn the real binary, speak the protocol over pipes, assert the
      answers — a protocol that only works in-process proves nothing
- [ ] `cargo xtask fmt`, then `cargo xtask gate` — every section green
- [ ] A real session driven by hand against a scratch project
- [ ] Re-run the whole suite under a runner-style short `TEMP`, since that is what broke
      CI last time

## Documentation

- [ ] `README.md` — what it is, how to point an agent at it
- [ ] `docs/architecture/overview.md` — a third entry point changes the system's shape
- [ ] `docs/__arch__/ROADMAP.md` and `open-questions.md` — the decision and its limits

## Completion

- [ ] Checklist filled with `+`/`-`
- [ ] Merge into `main` fast-forward, push to `origin/main`
- [ ] Final report
