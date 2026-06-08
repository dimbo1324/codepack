# Project Exporter Desktop v5.0

A Windows-friendly desktop utility for preparing a project for ChatGPT, Codex, code review, refactoring and security triage. It builds a safe project copy, generates AI-ready reports/prompts, creates an export plan before copying, and writes one ZIP or a logically split ZIP set when the package is large.

## Run from source

```powershell
python main.py
```

or, from the repository root:

```powershell
$env:PYTHONPATH = "src"
python -m project_exporter_desktop
```

## Build a portable Windows EXE

```powershell
build_windows_exe.bat
```

The script installs PyInstaller and creates `dist\ProjectExporterDesktop.exe`.

## Main features

- Safe Export modes: `safe`, `balanced`, `full`.
- Full **Export Plan** before the export starts: included files, excluded files, sensitive warnings, large files and estimated size.
- `.exportignore` support with a common `.gitignore`-style subset.
- GUI Include/Exclude Rules editor.
- Text dump size is unlimited by default; a manual limit can be enabled in the UI.
- Logical ZIP planning before archive creation; large projects are not first written as one huge temporary ZIP.
- Logical archive splitting into metadata/reports/AI context/source/tests/docs/config/assets/data/other groups.
- Restore helper files for split archives: `ARCHIVE_SET_MANIFEST.json`, `RESTORE_INSTRUCTIONS.md`, `restore_archives.py`.
- Diff Export modes: full project, uncommitted changes, changed-since-ref and between-refs.
- Incremental Export mode based on the previous successful export baseline.
- Export Comparison Report for incremental exports.
- AI handoff material: `AI_CONTEXT/`, `AI_PROMPTS/`, `CUSTOM_PROMPT.md`.
- Prompt Builder in the GUI.
- Project health score, architecture map, dependency intelligence and large-file reports.
- Local `REPORT_DASHBOARD.html` dashboard.
- Security outputs in human-readable, JSON and SARIF formats.
- Built-in report plugin registry via `REPORT_PLUGINS.json`.
- Export history saved locally.
- Settings import/export/reset.
- One-click Codex Package with safe defaults.
- Editable local export profile presets via `~/.project_exporter_profiles.json`.

## Safe sharing defaults

The default mode is `safe`. It skips common high-risk files during copy, including `.env`, private keys, local databases, dumps and nested archives. This is a safety layer, not a replacement for manual review. Always check:

```text
reports/insights/06_security_scan.txt
reports/insights/06_security_scan.json
reports/insights/REPORT_DASHBOARD.html
```

before sharing a bundle externally.

## `.exportignore`

Create `.exportignore` in your project root, or use the **Создать .exportignore** button.

Example:

```gitignore
node_modules/
.git/
dist/
build/
*.log
*.zip
*.db
.env*
private/
large-assets/
!README.md
!docs/
```

The built-in safety policy is still applied after custom rules, so `Always include` does not silently bypass Safe Export protection.

## Archive behaviour

- If the planned archive input is under the configured target, one ZIP is created on the Desktop.
- If the package is large, a Desktop folder named `*_archives` is created.
- Every part is grouped logically by file role, not randomly.
- The hard limit defaults to 512 MB per ZIP part.
- If a single file is itself larger than the limit, it is documented as oversized in the archive manifest.

## Tests

The app uses only the Python standard library at runtime. Tests use `unittest`:

```powershell
$env:PYTHONPATH = "src"
python -m unittest discover -s tests
```

## Custom export profile presets

Use the **Profiles JSON** button in the UI to create/open `~/.project_exporter_profiles.json`. Custom profiles can set a built-in `base_profile` plus safe-export, Git patch, ZIP limit and text-limit defaults. Restart the app after editing the file so the combobox reloads the profile list.
