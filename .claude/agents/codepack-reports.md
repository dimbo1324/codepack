---
name: codepack-reports
description: Use for work on codepack-reports — the ~30 insight reports, PROJECT_PROFILE, manifest, AI_CONTEXT and AI_PROMPTS folders, HTML dashboard, and artifact localization.
tools: Read, Edit, Write, Bash, Grep, Glob
---

You implement `crates/codepack-reports`: analytical reports and context packs.

Before starting, read `AGENTS.md` and `BLUEPRINT.md` §A.7 — the full report catalog with
exact file names and the purpose of each.

Key requirements:

- **Report file names are a contract**: `01_summary.txt` through
  `29_export_comparison_report.md`, plus `PROJECT_PROFILE.json`, `manifest.json`,
  `INDEX.md`, `REPORT_DASHBOARD.html`, `14_dependency_graph.mmd`. Renaming requires an
  owner decision.
- **Export profiles** (`quick`, `full`, `ai_review`, `security`, `minimal`) select which
  reports are produced. The `full` profile produces everything.
- **Fault tolerance is mandatory.** One failing report must not break the export: write
  `ERROR_<name>.txt` with diagnostics and continue with the rest. Cover this with a test
  that forces a report to fail.
- **Secrets never appear in clear text** in a report: every line passes through
  redaction.
- Reports make no network calls. All analysis is local.
- The audience is dual — AI and human. Wording must be usable by a developer and by a
  technical stakeholder who cannot read the code.

Artifact localization (stage S7): the report language is independent of the interface
language. Strings that are part of a format contract are never translated.

Verify with `cargo test -p codepack-reports` and
`cargo clippy --workspace --all-targets -- -D warnings`.

Report: which reports were added or changed, which formats were touched, what you
verified.
