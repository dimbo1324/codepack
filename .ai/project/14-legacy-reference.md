# Legacy Reference: The Previous Python Implementation

The previous product version (Project Exporter Desktop 1.0.1, Python + PySide6,
Windows-only) is preserved as an archive:

```text
docs/__arch__/codepack-main.zip
```

It is a **behavioral reference, not a code model**. The new implementation must
reproduce its results, never its structure.

## When to consult the archive

- You need exact constant values: extension sets, sensitive names and suffixes, ignored
  directory lists, safety-mode rules.
- You need an exact artifact format: `manifest.json`, `PROJECT_PROFILE.json`, SARIF,
  report section names and ordering.
- Behavior is ambiguous and `BLUEPRINT.md` does not answer it decisively.
- You need a golden reference for a parity test.

## How to work with it

1. Look in `BLUEPRINT.md` first — it documents the legacy logic in full. The archive is
   for when literal precision is required.
2. Extract into a **temporary directory outside the repository**. Never extract into the
   working tree and never commit extracted content.
3. Take facts only: values, formats, step ordering.
4. Do not carry the Python architecture into Rust. Layering and module organization are
   defined by `ROADMAP.md` and the domain rules module.

## Where things lived in the legacy version

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

- Known weaknesses: keyword-only secret detection is strengthened in stage S3, flat JSON
  storage becomes SQLite in S5, Windows-specific assumptions are removed.
- Russian interface strings go through the new localization system. Only strings that
  are part of an artifact format contract are preserved verbatim.
