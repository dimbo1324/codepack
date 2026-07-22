---
name: legacy-lookup
description: Use when you need the exact behavior, constants, or artifact format of the old Python implementation stored in docs/__arch__/codepack-main.zip.
---

# Consulting the Legacy Implementation

The previous version (Project Exporter Desktop 1.0.1, Python + PySide6) is archived at:

```text
docs/__arch__/codepack-main.zip
```

It is a **behavioral reference, not a code model**.

## Check BLUEPRINT first

`BLUEPRINT.md` documents the legacy logic in full: the eight-step pipeline, all 25
configuration fields, safety modes with their suffix sets, stack detection, the catalog
of ~30 reports, archiving parameters, and formulas. **In most cases the answer is there.**

Reach for the archive only when literal precision is required: a complete constant list,
an exact artifact layout, the ordering of report sections.

## How to work with the archive

1. Extract into a **temporary directory outside the repository**:

   ```powershell
   $tmp = Join-Path $env:TEMP "codepack-legacy"
   Expand-Archive -Path docs\__arch__\codepack-main.zip -DestinationPath $tmp -Force
   ```

2. Never extract into the working tree and never commit extracted content.
3. Take facts only: constant values, formats, step ordering.
4. Do not carry the Python architecture into Rust. Layering is defined by `ROADMAP.md`
   and `.ai/project/12-domain-rules.md`.

## Where things lived

| What you need | File in the archive |
|---|---|
| Constants, extension sets, sensitive names | `src/project_exporter_desktop/constants.py` |
| Configuration fields and normalization | `config.py` |
| Export pipeline | `services/exporter.py` |
| Safety modes | `services/export_policy.py` |
| Secret redaction | `utils/text_utils.py` |
| Security scanner | `reports/insights/security.py` |
| Stack detection | `services/stack_detector.py` |
| Differential export | `services/diff_service.py` |
| Archiving and splitting | `services/archive_service.py` |
| Report catalog | `reports/insights/orchestrator.py` |
| Token estimation | `utils/token_counter.py` |

## What not to carry over

- Known weaknesses: keyword-only secret detection is strengthened in S3, flat JSON
  storage becomes SQLite in S5, Windows-specific assumptions are removed.
- Russian interface strings go through the new localization system. Only strings that
  are part of an artifact format contract are preserved verbatim.
