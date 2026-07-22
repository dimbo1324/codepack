---
name: ci-fix
description: Use when a local check or CI job is failing — reproduce, find the root cause, apply the minimal correct fix, and re-verify.
---

# Fixing a Failing Check

## Procedure

1. **Reproduce** with a minimal command. No diagnosis without reproduction.

   ```powershell
   cargo xtask gate --quick
   ```

   Or target one layer: `cargo fmt --all --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
   `cargo xtask sync-agents --check`.

2. **Localize the layer:** formatting, clippy, a test, the build, a dependency, or a
   platform difference.
3. **Find the cause, not the symptom.** A failing test usually means a code defect.
4. **Apply the minimal fix** without expanding scope.
5. **Re-verify** with the same command, then with the full gate.

## Forbidden

- Deleting, `#[ignore]`-ing, or weakening tests to make the gate green.
- Lowering secret-detector accuracy thresholds.
- Silencing clippy with `#[allow(...)]` instead of fixing it, except with a justification
  in an adjacent comment.
- Suppressing output so a check appears to pass.
- Hand-editing `AGENTS.md` when it drifts — run the sync command.

## Common local-versus-CI differences

- Line endings: the repository normalizes text to LF.
- Path case and length on Windows; antivirus locking temp directories.
- Missing `webkit2gtk` and related libraries on the Linux runner.
- Missing Xcode Command Line Tools on the macOS runner.
- `AGENTS.md` drifting from the `.ai/` modules — fixed by `cargo xtask sync-agents`.

## Report

The reproduction command, the root cause, the fix applied, and the result of the re-run.
If the cause was not found, say so honestly and list what was checked and ruled out.
