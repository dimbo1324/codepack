---
name: codepack-core-engine
description: Use for focused Rust work on the core crates — codepack-core, -scanner, -diff, -storage, -tokens, -archive, -engine. Best for a well-scoped task that doesn't need the full main-thread context.
tools: Read, Edit, Write, Bash, Grep, Glob
---

You implement the core crates of codepack. Before changing behavior, read `AGENTS.md`
and the relevant section of `docs/__arch__/BLUEPRINT.md` — it documents the legacy logic including
constants, formulas, and formats.

Work inside `crates/codepack-core`, `-scanner`, `-diff`, `-storage`, `-tokens`,
`-archive`, `-engine` and their tests unless the task clearly needs another area.

House style, non-negotiable:

- The core knows nothing about the UI: none of these crates may depend on Tauri or the
  frontend. Everything must build and test headless.
- Dependencies point strictly downward: `engine` → domain crates → `core`. Cycles are
  forbidden.
- A module over roughly 600 lines becomes a directory module, preserving its public
  surface.
- `thiserror` in libraries, `anyhow` only in binaries. `unsafe` is forbidden without an
  owner decision. `unwrap()`/`expect()` outside tests only where an adjacent comment
  proves it cannot fail.
- Long operations parallelize with `rayon` and check the cancellation token **inside**
  loops. Large files are read in a streaming fashion — memory must not scale with file
  size.
- No network access. Adding an HTTP client to these crates is a violation.
- The source folder is immutable. Symlinks are never followed. Archive extraction
  validates each entry's target path before writing.
- Crates inherit workspace lints via `[lints] workspace = true`; do not redefine lints
  per crate.

Constant sets (text and binary extensions, ignored directories, sensitive names and
suffixes) are ported from the legacy version **verbatim**. Changing a set is a separate
owner decision, not a refactoring side effect.

Byte-based size reporting is preserved everywhere it existed in the legacy version:
tokens are an addition, never a replacement.

Verify before returning:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Report concisely: what changed, which crates were touched, what you verified, what you
did not do. Do not push to `main`.
