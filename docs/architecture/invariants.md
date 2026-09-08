# Invariants: what must never break

> This registry is binding. Breaking an invariant is a defect, not a trade-off.
> Only the owner can change one, and that decision is recorded — with its date and
> reasoning — in the project's decision log.

## I1. Privacy is absolute

All analysis runs locally. No crate reaches the network. The single exception is the
stage S13 integration, and only on an explicit user action. Adding an HTTP client to any
other crate is a violation.

**Why.** The product handles other people's source code and secrets. Trusting it rests
entirely on the data not going anywhere.

**How it is enforced (since 2026-07-27, strengthened 2026-09-06).** This stopped being
text and became a mechanism: the `network isolation` step of `cargo xtask gate` reads
every crate manifest and fails if an HTTP client is declared. The reason is the shape of
the failure — a crate that gains an HTTP client behaves identically until the day it makes
a request, so "someone will catch it in review" does not work here. Same approach as the
webview's isolation: not a convention, a mechanism
(`crates/xtask/src/network_isolation.rs`).

Since 2026-09-06 the check allows **no** exception rather than one. S13's API path moved
to `codepack-ai-api`, a crate this repository contains but the workspace excludes, so the
transport is not in the product at all — not in a binary, not in `Cargo.lock`, not in
`cargo deny`'s graph. The check also refuses a member that depends on that crate, which is
the one way to bring the client back without naming any client. Owner decision 2026-09-06,
Q41; the reason was cross-platform rather than tidiness (`keyring` wants Secret Service on
Linux and was compiled there for code no user could reach).

The checker subtracts `workspace.exclude` from the member globs, as cargo does. It did not,
and the first thing the exclusion produced was the check reporting the product in violation
because of a crate the product does not build.

## I2. The source is immutable

An export never writes, renames or deletes anything inside the source project folder.
All work happens on a copy in the staging directory.

**Why.** People run this against their working project. Corrupting the source is
unacceptable under any circumstances, cancellation and failure included.

**How it is enforced (since 2026-09-06).** In `codepack_engine::run_export`, before the
pipeline computes a single path — the one layer both front ends go through. An output
directory that resolves inside the source root fails with
`EngineError::OutputInsideSource`.

It used to be checked only in `codepack-cli`, which left the desktop shell — which calls
the engine directly — free to stage a bundle inside the user's own working tree, pick it
up as a source on the next run, and, with `keep_staging_folder = false`, recursively
delete a directory inside their project.

The comparison runs on a *prospective* path
(`codepack_core::validate_destination_outside`): the destination is resolved through its
longest existing ancestor rather than created first. A check that runs `create_dir_all`
before refusing leaves a stray directory inside the source tree, which is this invariant
broken by the code meant to hold it — the CLI did exactly that until this date. The same
function now serves all three callers (engine, CLI `--out`, `codepack-sanitize`), where
the rule was previously written three times and behaved differently in each.

## I3. A secret never leaves the redactor

The value of a detected secret never reaches a log, a report, the history, the database,
an error message or the clipboard in the clear. Redaction is applied before anything is
written.

**Why.** A tool that logs the secrets it finds becomes a source of leaks itself.

**How it is enforced (strengthened 2026-09-07).** Four mechanisms, because this invariant
has the most surfaces of any of them and each was broken at least once:

- `crates/xtask/src/report_redaction.rs`, a gate step: raw project content is reachable
  only through `text::read_text_unredacted`, and every report calling it must be declared
  there with a sentence saying why what it reads is safe. Adding the call without the
  entry fails the build.
- `codepack_engine::LogLine` is constructible only through `LogLine::of`, which redacts —
  the activity log cannot be written to except through the redactor.
- The progress channel's `log`/`log_info` closures redact before sending. Until
  2026-09-07 they did not, and a secret reached the CLI's stderr and the desktop window
  live, in the clear. Any new consumer of that channel inherits the fix.
- `crates/codepack-security/tests/i3_no_secret_leak.rs` asserts no serialized `Finding`
  contains a substring of the original value, and `tests/utf8_boundaries.rs` proves the
  redactor does not panic — a panic mid-redaction leaves the caller holding the
  unredacted input.

## I4. Byte figures are preserved

Size in bytes is reported everywhere the previous version reported it. Tokens are an
additional metric alongside, never a replacement.

**Why.** A direct owner decision (2026-07-22): bytes are what people read, and what
tells you the real volume of data.

**How it is enforced.** `codepack-tokens` owns the byte formatting, ported verbatim from
the previous implementation, and `tests/golden/` compares whole generated artifacts
against that implementation's real recorded output. A byte figure that changed shape, or
disappeared in favour of a token count, fails the comparison rather than being noticed by
a reader.

## I5. Artifact formats stay backward compatible

Report file names and the structure of `manifest.json`, `PROJECT_PROFILE.json`,
`06_security_scan.json`, SARIF 2.1.0 and `ARCHIVE_SET_MANIFEST.json` are a contract.
Changing one requires bumping `schema_version` and recording the decision.

**Why.** These artifacts are consumed by other tools and by people; changing a format
quietly breaks someone else's process.

**How it is enforced.** The same golden comparison in `tests/golden/`, plus every
artifact writer carrying its own `schema_version`. The version is the deliberate part: a
format may change, but not silently — the number moves and the decision is recorded in
`docs/__arch__/open-questions.md`. Changing a field without moving it is what the golden
test exists to make loud.

## I6. Cancelling never corrupts state

Every long operation can be interrupted at any moment. A cancelled or failed run does
not overwrite the snapshot baseline and leaves behind no partial data presented as
complete.

**Why.** Otherwise the next differential export produces a wrong answer.

**How it is enforced.** Cancellation is checked inside each step's loops rather than only
between steps, and the archive writers build into a staging file that is moved into place
only once complete — a cancelled run leaves last week's archive exactly as it found it.
Proven by `codepack-engine/tests/cancellation.rs`, and in `codepack-archive` by
`a_cancelled_run_leaves_no_half_written_archive_behind` and
`a_failed_run_leaves_an_existing_archive_of_the_same_name_untouched`. One limit is known
and documented rather than hidden: cancelling mid-copy of a single very large archive
member does not interrupt that member, characterised by
`tests/cancellation_mid_file.rs`.

## I7. Walking and extraction are safe

Symlinks are never dereferenced while walking a tree. When extracting an archive, each
member's target path is validated before anything is written (path-traversal safety).

**Why.** Otherwise a specially crafted project or archive escapes the destination
directory.

**How it is enforced.** One primitive, `codepack_core::safe_join`, used by every path
that resolves an outside-supplied path component — archive extraction and, since
2026-09-07, `codepack init --hook`'s `core.hooksPath`. Symlinks are refused at the walk
and never packed. `codepack-archive/tests/security.rs` feeds the extractor every
malicious member shape and asserts nothing is written outside the destination; the
symlink half is covered by `#[cfg(unix)]` tests that run on the macOS and Linux gate legs.
Paths are compared with `Path::components`, never by splitting on a separator — a
backslash is a path separator on Windows only, and that difference had already hidden one
traversal defect for months (Q21).

## I8. The core does not depend on the interface

No `codepack-*` crate depends on Tauri or on the frontend; the whole core builds and
tests headless. Dependencies point strictly downward, and cycles are forbidden.

**Why.** The CLI, automation and testability all rest on this — and mixing the layers is
what made the previous version Windows-only.

**How it is enforced.** Structurally: `codepack-desktop` is a workspace member that
depends on the crates, and no crate depends back. `cargo xtask gate` builds and tests the
whole workspace headless on three runners, so a crate that grew a Tauri dependency would
fail to build where no webview exists. The dependency direction itself is read
mechanically by `crates/xtask/src/network_isolation.rs`, which parses every member's
manifest — the same walk that enforces I1.

## I9. Detector quality thresholds are never lowered

The precision/recall thresholds of `codepack-security`'s corpus test are not lowered to
make a build green. A drop in recall is a defect in the detector, not a reason to edit
the test.

**Why.** Secret detection is the product's central value; degrading it quietly is more
dangerous than a red build.

**How it is enforced.** `crates/codepack-security/tests/corpus.rs` scores every detector
against a labelled corpus of positives and negatives and asserts the metrics directly, so
a regression fails the gate with the numbers in the message rather than needing someone
to read a log. Its own module doc states the rule in the imperative — lowering a threshold
to make the test pass is forbidden, and a recall drop is a defect to report. The corpus is
append-only in spirit: cases are added, never removed to make a number look better.
