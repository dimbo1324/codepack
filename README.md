# Project Exporter Desktop

Desktop utility for Windows 11 that creates an AI-ready project export ZIP for a selected project folder.

The generated archive contains a copied project folder plus human-readable and machine-readable reports that help you, ChatGPT, Codex, or a reviewer understand the project quickly and safely.

## What the export contains

- copied project folder, excluding cache/heavy folders such as `.git`, `node_modules`, `__pycache__`, `.venv`, `dist`, `build`, `.next`, `.turbo`, and user-defined exclusions;
- `manifest.json` with export metadata;
- `PROJECT_PROFILE.json` with project type, detected stack, commands, entrypoints, capabilities, and risk level;
- `INDEX.md` as a table of contents;
- structure, Git, and text-dump reports;
- dependency, scripts, Docker, config, security, TODO/FIXME, metrics, API, frontend, backend, architecture, key-files, and refactoring reports;
- Mermaid dependency graph;
- `reports/insights/AI_CONTEXT/` folder with a ready-to-use Codex/ChatGPT handoff pack.

## Export profiles

The UI supports several export profiles:

- `quick` — compact but useful overview;
- `full` — all available reports;
- `ai_review` — reports optimised for ChatGPT/Codex project analysis;
- `security` — security/config/dependency/Git/code-quality oriented export;
- `minimal` — lightweight overview for sharing.

The default profile is `full`.

## Requirements

- Windows 11;
- Python 3.11+ recommended;
- no third-party Python packages required.

The application intentionally uses only the Python standard library.

## How to run

### Option 1 — double-click the BAT file

1. Extract this folder anywhere.
2. Double-click `SaveFilesContent.bat`.
3. Select the root folder of the project you want to export.
4. Choose an export profile.
5. Press `Создать экспорт`.
6. The resulting ZIP will appear on your Desktop.

### Option 2 — run from PowerShell

```powershell
py -3 main.py
```

or:

```powershell
python main.py
```

## Project structure

```text
home-scripts/
├── main.py
├── SaveFilesContent.bat
├── README.md
├── .gitignore
├── assets/
│   └── ICO.ico
└── src/
    └── project_exporter_desktop/
        ├── constants.py
        ├── config.py
        ├── models.py
        ├── main.py
        ├── services/
        ├── reports/
        ├── reports/insights/
        ├── ui/
        └── utils/
```

## Development checks

```powershell
py -3 -m compileall main.py src
py -3 main.py
```

The application runs read-only Git commands only. It does not switch branches, commit files, or modify the selected source repository.
