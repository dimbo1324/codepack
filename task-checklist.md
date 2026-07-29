# Task Checklist

**Task:** Seven items from the owner, 2026-07-30, on one branch.

**Date:** 2026-07-30
**Branch:** feat/archive-formats-docs-split-ui-polish

Owner approved, once green: push, merge into `main`, re-release.

## 1 — Archive format is a choice, ZIP by default

- [+] Correction understood: the earlier "7z" instruction was a slip; `.zip` is the
      default everywhere an archive is made
- [+] `ArchiveFormat` in `codepack-archive`; `Config::archive_format` defaults to `zip`,
      so every existing config keeps producing exactly what it produced before
- [+] Real ZIP writer beside the 7z one, at the same settings the export pipeline uses
- [+] Both paths honour it: the export bundle (single archive, split parts, and the
      restore files that name the format) and the sterile copy
- [+] `sanitize` infers the container from the file extension; `--archive-format` wins
- [+] RAR: listed, dimmed in the interface, explains itself on click, and refused in code
      **before anything is written**, naming the alternatives
- [+] Tests: both containers round-trip; identical content from identical input; ZIP is
      the default; RAR creates nothing; export honours the flag end to end

## 2 — Internal vs external documents

- [+] `BLUEPRINT.md`, `ROADMAP.md`, `open-questions.md` → `docs/__arch__/`, Russian
- [+] `docs/README.md` (an index linking internal docs) moved there too
- [+] 79 files' references rewritten; verified idempotent and free of double prefixes
- [+] External set in English: `README.md`, `docs/architecture/overview.md`,
      `docs/architecture/invariants.md`
- [+] `.ai/project/10-project-map.md` — the split and a rewritten language policy
- [+] `.ai/project/13-progress-tracking.md` — duty to keep `README.md` current
- [+] `.ai/CHANGELOG.md` entry; `AGENTS.md` regenerated
- [+] Both assistant mirrors (`.claude/`, `.codex/`) updated in the same task
- [+] `AGENTS.md` went over its 30 KiB budget; two lookup sections moved to the extended
      command-reference module rather than raising the limit

## 3 — Frontend and translations

- [+] `Segmented` gained an `unavailable` option state: dimmed, focusable, explains
      itself on click instead of being hidden or silently selectable
- [+] Format picker on the sterile-copy page and in Settings, from one shared definition
- [+] Choosing a format retargets an already-picked file, so the extension never
      contradicts the container
- [+] Audited `ru.ts`: 17 of 321 entries contained latin text, and **all 17 were
      placeholders or proper nouns** (`{count}`, `Git-`, `AI-`) — the translation file
      was already clean
- [+] The real gap was backend prose reaching the UI untranslated. `FileOutcome` now
      carries a stable `detail_kind` token the frontend translates, with proper nouns
      (a language, a formatter) passed through untranslated because they are names
- [-] Findings text (`finding.message`) and the ~30 report artifacts remain English.
      Not fixed here: those strings are an artifact contract (invariant I5) and their
      localization is the long-standing open question Q12, not this task

## 4 — Screenshots in README

- [-] **Not done, deliberately.** Window capture on Windows copies a screen region, not
      the window's own content, so whatever sits on top lands in the file. The first
      attempt captured an unrelated private messenger window; the file was deleted at
      once and never entered git. Putting that in a public repository is not a risk worth
      taking for a screenshot. Recorded as Q33 with two ways forward

## 5 — External documentation reachable from README

- [+] `README.md` is the hub: a table of contents, and links to both external documents
- [+] It links to no internal document — that is what "internal" now means

## 6 — GitHub CI failure

- [+] Root cause: `explain` compared a canonicalized file path against the project root
      as given. On the GitHub runner those are `runneradmin` and `RUNNER~1` — an 8.3
      short name, which no amount of case-folding reconciles
- [+] Fixed by resolving both sides the same way, through the longest existing ancestor
- [+] Two regression tests
- [-] Could not reproduce locally: this machine has no 8.3 short names (verified). The
      tests assert the property rather than the environment

## 7 — More tests

- [+] Cross-format equivalence: same input, same members and bytes in both containers
- [+] `member_name` cannot produce an escaping entry
- [+] `ArchiveFormat` parsing, defaults, and agreement with the core valid-set lists
- [+] CLI end to end for both containers, the extension-vs-flag precedence, and RAR

## Verification

- [+] `cargo fmt --all --check`
- [+] `cargo clippy --workspace --all-targets -- -D warnings`
- [+] `cargo xtask gate` — all eight sections
- [+] `cargo deny check` (part of the gate)
- [+] Push, merge to `main`, re-release

---

## Next task

Not yet chosen. Start with the orientation ritual from
`.ai/project/13-progress-tracking.md`.
