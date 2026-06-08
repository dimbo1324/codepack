# Security Notes

Project Exporter Desktop is a local desktop utility. It does not upload anything by itself, but the ZIP files it creates may later be shared with AI tools or reviewers. Treat every export as potentially sensitive.

## Default protections

- Safe Export is enabled by default.
- Common secrets and credential files are skipped during copy.
- `.env.example` / `.env.sample` are allowed because they usually contain placeholders.
- Text reports redact obvious secrets.
- Git patches are disabled by default.
- The security scanner writes human-readable, JSON and SARIF outputs.

## What Safe Export does not guarantee

- It cannot prove that every secret was removed.
- It cannot safely inspect all proprietary binary formats.
- It cannot know whether a value is real or a placeholder.
- `full` mode can include sensitive files and should be used only locally/private.

## Recommended sharing checklist

1. Use `safe` mode.
2. Keep Git patch disabled unless you really need it.
3. Review `reports/insights/REPORT_DASHBOARD.html`.
4. Review `reports/insights/06_security_scan.txt`.
5. Review `reports/insights/06_security_scan.json` for machine-readable findings.
6. Check `reports/insights/28_export_plan.md` before sharing.
7. Delete the export if you accidentally included secrets and rotate exposed credentials.

## `.exportignore`

Use `.exportignore` for project-specific exclusions such as local databases, generated files, large assets, private folders and archives.

Safe Export is applied after custom rules, so custom include rules do not silently bypass high-risk safety checks.
