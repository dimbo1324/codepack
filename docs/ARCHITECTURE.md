# Architecture

Project Exporter Desktop is intentionally layered and standard-library-only at runtime.

```text
main.py / project_exporter_desktop.main
  -> ui.app_window.App
    -> services.ProjectExporter
      -> export_plan
      -> export_ignore
      -> incremental
      -> copy_service
      -> archive_service
      -> git_diff
      -> reports/*
      -> utils/*
```

## Responsibilities

- `ui/` contains Tkinter state, validation, progress display, settings import/export and user feedback.
- `services/` contains business workflows: export planning, `.exportignore` parsing, incremental state, copy, archive creation, Git diff selection, prompt building and export history.
- `reports/` contains deterministic report generators.
- `reports/insights/` contains higher-level project analysis and dashboard/security outputs.
- `reports/insights/plugins.py` defines the built-in report plugin descriptor used by the orchestrator.
- `utils/` contains reusable path, text, inventory and time helpers.
- `models.py` contains dataclasses shared across layers.
- `constants.py` contains static lists and defaults.
- `config.py` persists user settings and migrates older settings.

## Pipeline

1. Validate project root and UI settings.
2. Resolve Git diff selection.
3. Resolve incremental selection from the local baseline.
4. Load `.exportignore` and GUI include/exclude rules.
5. Build an Export Plan and ask the user for confirmation.
6. Copy selected files into a staging folder.
7. Generate structure, Git, text dump and insight reports.
8. Generate AI context, prompts, dashboard and machine-readable security outputs.
9. Build an archive plan from the staging folder.
10. Create either one ZIP or a logical split archive set.
11. Write history and update the incremental baseline after a clean successful run.

## Dependency direction

UI may depend on services. Services may depend on reports, models and utils. Reports may depend on models/utils/constants. Utility modules should not depend on UI or services.

## Extension points

- Add a report by creating a module in `reports/insights/` and registering a `ReportPlugin` in `orchestrator.py`.
- Add a copy-time safety rule in `services/export_policy.py`.
- Add custom export-ignore behaviour in `services/export_ignore.py`.
- Add a new archive grouping rule in `classify_archive_group()` inside `services/archive_service.py`.
- Add a UI setting in `Config`, `App._load_config_to_ui()` and `App._config_from_ui()`.
