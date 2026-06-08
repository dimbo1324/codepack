# Architecture

The application is intentionally layered:

```text
main.py / project_exporter_desktop.main
  -> ui.app_window.App
    -> services.ProjectExporter
      -> copy_service
      -> archive_service
      -> git_diff
      -> risk_preview
      -> reports/*
      -> utils/*
```

## Responsibilities

- `ui/` contains Tkinter state, validation and user feedback.
- `services/` contains long-running business workflows: copy, archive creation, Git diff selection, risk preview and export history.
- `reports/` contains deterministic report generators.
- `reports/insights/` contains higher-level project analysis.
- `utils/` contains reusable path, text, inventory and time helpers.
- `models.py` contains dataclasses shared across layers.
- `constants.py` contains static lists and defaults.
- `config.py` persists user settings and migrates older settings.

## Dependency direction

UI may depend on services. Services may depend on reports, models and utils. Reports may depend on models/utils/constants. Utility modules should not depend on UI or services.

## Extension points

- Add a report by creating a module in `reports/insights/` and registering it in `orchestrator.py`.
- Add a copy-time safety rule in `services/export_policy.py`.
- Add a new archive grouping rule in `_classify_archive_group()` inside `services/archive_service.py`.
- Add a UI setting in `Config`, `App._load_config_to_ui()` and `App._config_from_ui()`.
