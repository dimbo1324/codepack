# Task Checklist

**Task:** Verify the finished orchestrator work, teach the analyser OS/kernel source
stacks (C, Assembly, Rust, Shell, Python, Makefile), and make the installer register in
Windows "Uninstall a program" with a Start Menu and desktop shortcut.
**Date:** 2026-07-26
**Branch:** feat/systems-stack-and-installer-registration

Owner asked for autonomous execution: no confirmation prompts. Anything that genuinely
needs a decision goes into `docs/decisions/open-questions.md`, not into a blocking
question.

## Task 1 — verify what was finished while the session was interrupted

- [ ] `main` in sync with `origin/main`, history reviewed
- [ ] `cargo xtask gate` green end to end
- [ ] Orchestrator catalog loads, `selftest` passes

## Task 2 — OS / kernel source support

The target is a tree like `torvalds/linux`: C, Assembly, Rust, Shell, Python, Makefile.
Three separate gaps, all real:

**2a. Assembly and friends are not even treated as text.** `classify.rs` has `c`, `h`,
`mk` but no `s`/`asm`, no `dts`/`dtsi`, no `lds`. The kernel's assembly and device trees
would be classified as non-text and dropped from the text dump entirely — the worst of
the three gaps, because the content silently disappears rather than being mislabelled.

- [ ] `TEXT_EXTENSIONS` gains assembly, device-tree, linker-script and build-input
      extensions
- [ ] `TEXT_FILENAMES_WITHOUT_EXTENSION` gains `kconfig`, `kbuild`, `gnumakefile` and the
      other extensionless files a kernel tree is full of
- [ ] Recorded as an owner decision: `12-domain-rules.md` says legacy constant sets change
      only by explicit decision, never as a side effect

**2b. No language is reported for them.** `LANGUAGE_BY_EXTENSION` has no Assembly, and
extensionless `Makefile`/`Kconfig` can never match because the map is keyed on extension.

- [ ] Assembly, device tree, linker script, Perl, awk, CMake added to the language map
- [ ] Filename-based language detection for `Makefile`/`Kconfig`/`Kbuild`, added **beside**
      `extension_key` rather than inside it, so `by_extension` statistics — and therefore
      golden parity — are untouched
- [ ] Verified: golden fixtures contain no `.c`/`.S`/`Makefile`, so none of this can move
      legacy parity (checked before writing any of it)

**2c. The stack detector cannot recognise a kernel or any C project.** Marker-file rules
cover twelve ecosystems, none of them C.

- [ ] `Linux kernel` rule (`Kconfig` + `Kbuild`/`MAINTAINERS`), with the build output it
      should prune
- [ ] `C / Make`, `C / CMake`, `C / Meson` rules for ordinary C projects
- [ ] `detect_stack` in `codepack-reports` learns the same markers so
      `PROJECT_PROFILE.json` and the reports agree with the scanner
- [ ] Tests, including a synthetic kernel-shaped tree

## Task 3 — installer registers as a normal Windows program

- [ ] Bundle metadata filled in: `publisher`, `copyright`, `shortDescription`,
      `homepage`, `category` — these are what Control Panel actually displays
- [ ] Start Menu shortcut and desktop shortcut created by the installer
- [ ] Built, then **actually installed**, and verified: the Uninstall registry key exists
      with `DisplayName`/`DisplayVersion`/`Publisher`/`UninstallString`, the shortcuts
      exist on disk, and the entry is visible in the programs list
- [ ] Uninstall verified too: it removes the key and the shortcuts rather than orphaning
      them

## Verification

- [ ] `cargo xtask gate` green
- [ ] `cargo test -p codepack-engine --test golden` 3/3 — parity unmoved
- [ ] A real kernel-shaped tree analysed end to end through the CLI, and the reports read
      correctly (stack, language breakdown, no assembly dropped)
- [ ] Installer install/verify/uninstall cycle done on this machine
- [ ] Independent review of the diff

## Completion

- [ ] `docs/decisions/open-questions.md`: the constant-set decision, plus anything that
      needs the owner later
- [ ] `docs/architecture/overview.md` and `ROADMAP.md` updated where the shape changed
- [ ] Checklist filled `+`/`-`, final report in Russian
- [ ] Merge to `main`, push
