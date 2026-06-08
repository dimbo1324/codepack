# Project Exporter Desktop v4.0

A safe Windows-friendly desktop utility for exporting a project into an AI/code-review handoff package. It creates a project copy, generated reports, AI context files, prompt templates and a ZIP archive. If the ZIP exceeds the configured limit, the app creates a Desktop folder with logically grouped archive parts.

## Run

```powershell
python main.py
```

or:

```powershell
python -m project_exporter_desktop
```

For direct package imports from the repository root, use:

```powershell
$env:PYTHONPATH = "src"
python -m project_exporter_desktop
```

## Main features

- Safe Export modes: `safe`, `balanced`, `full`.
- Pre-export risk preview before the copy starts.
- Text dump size is unlimited by default; a manual limit can be enabled in the UI.
- Logical ZIP splitting when the final archive exceeds the configured size limit.
- Diff Export modes for full project, uncommitted changes, changed-since-ref and between-refs packages.
- AI handoff material: `AI_CONTEXT/` and `AI_PROMPTS/`.
- Project health score, architecture map, dependency intelligence and large-file reports.
- Export history saved locally.
- One-click Codex Package with safe defaults.
- Editable local export profile presets via `~/.project_exporter_profiles.json`.

## Safe sharing defaults

The default mode is `safe`. It skips common high-risk files during copy, including `.env`, private keys, local databases, dumps and nested archives. This is a safety layer, not a replacement for manual review. Always check `reports/insights/06_security_scan.txt` before sharing a bundle externally.

## Archive behaviour

- If the generated ZIP is smaller than or equal to the configured limit, one ZIP is created on the Desktop.
- If the ZIP is larger, the single archive is removed and a folder named `*_archives` is created on the Desktop.
- Files are grouped by meaning: metadata/reports/source/tests/docs/config/assets/data/other.
- `ARCHIVE_SET_MANIFEST.json` explains the generated archive set.

## Tests

The project uses only the Python standard library at runtime. The included tests use `unittest` and can be run without external packages:

```powershell
$env:PYTHONPATH = "src"
python -m unittest discover -s tests
```

## Custom export profile presets

Use the **Profiles JSON** button in the UI to create/open `~/.project_exporter_profiles.json`. Custom profiles can set a built-in `base_profile` plus safe-export, Git patch, ZIP limit and text-limit defaults. Restart the app after editing the file so the combobox reloads the profile list.
