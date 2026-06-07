
# Project Exporter Desktop

Desktop utility for Windows 11 that creates one export ZIP for a selected project folder.

The exported ZIP contains:

- a copied project folder;
- `manifest.json`;
- `INDEX.md`;
- reports with project structure, read-only Git information, text-file dump, dependency overview, security heuristics, TODO/FIXME markers, code metrics, Docker/configuration hints and an AI context pack.

## What is preserved from the original version

The refactor keeps the original business behaviour:

- `.git` and `node_modules` are always excluded from copied project content;
- user-defined ignored folders are additive and do not replace default ignores;
- Git commands are read-only and are executed against the original selected project folder;
- symbolic links are skipped;
- text dump can redact obvious secrets using simple heuristics;
- output is created on the Desktop by default.

## Requirements

- Windows 11
- Python 3.11+ recommended. The original header mentioned Python 3.14+, but the current code uses only the Python standard library and should run on modern Python 3 versions.
- No third-party Python packages are required.

## How to run

### Option 1 — double-click the BAT file

1. Extract this folder anywhere, for example next to the previous `File merger` folder.
2. Double-click `SaveFilesContent.bat`.
3. Select the root folder of the project you want to export.
4. Press `Создать экспорт`.
5. The resulting ZIP will appear on your Desktop.

### Option 2 — run from PowerShell

Open PowerShell in this folder and run:

```powershell
py -3 main.py
```

or:

```powershell
python main.py
```

## Project structure

```text
File merger decomposed/
├── main.py                         # Windows-friendly launcher
├── SaveFilesContent.bat             # Convenience launcher
├── README.md
├── .gitignore
├── assets/
│   └── ICO.ico
└── src/
    └── project_exporter_desktop/
        ├── constants.py             # App constants, detection sets, report descriptions
        ├── config.py                # Persistent user settings
        ├── models.py                # Dataclasses used by services/reports
        ├── main.py                  # Tkinter entry point
        ├── services/                # Copying, ZIP creation, export orchestration
        ├── reports/                 # Basic reports and metadata writers
        ├── reports/insights/        # Extended analytical reports
        ├── ui/                      # Tkinter window and event handlers
        └── utils/                   # Path, time, text and inventory helpers
```

## Notes for development

The application intentionally uses only the standard library. This keeps it portable on Windows 11 and avoids dependency installation.

Suggested quick checks after changes:

```powershell
py -3 -m compileall .
py -3 main.py
```

Do not run destructive Git commands from this app. The current implementation only reads Git state.
