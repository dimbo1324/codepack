# Security notes

## Threat model

The tool prepares project archives for AI/code-review workflows. The primary risk is accidental disclosure of secrets, local databases, dumps, private keys or sensitive Git patches.

## Controls

- `.git` is never copied.
- Symlinks are skipped to avoid escaping the selected project tree.
- Safe Export mode skips high-risk files at copy time.
- Git reports are read-only.
- Full Git patch export is disabled by default.
- Text dump and Git report output can redact obvious secret values.
- The security scan groups findings by severity/confidence.
- A pre-export preview is shown before the export starts.

## Operational guidance

Use `safe` mode for anything that may leave your machine. Use `balanced` only for trusted internal review. Use `full` only for private/local backups.

Before sharing externally, review:

- `manifest.json`
- `reports/insights/06_security_scan.txt`
- `reports/insights/25_large_files_report.md`
- generated archive contents

If a real secret was exported or committed, rotate it. Redaction prevents disclosure in generated reports; it does not make a leaked credential safe.
