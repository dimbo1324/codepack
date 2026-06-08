# Release Process

## Before release

```powershell
$env:PYTHONPATH = "src"
python -m compileall -q src tests
python -m unittest discover -s tests -v
```

## Optional Windows EXE build

```powershell
build_windows_exe.bat
```

The output executable is created under `dist\ProjectExporterDesktop.exe`.

## Manual smoke test

1. Run `python main.py`.
2. Select a small test project.
3. Keep Safe Export enabled.
4. Click **Создать экспорт**.
5. Confirm the Export Plan.
6. Open the result from Desktop.
7. Check these generated files:
   - `INDEX.md`
   - `manifest.json`
   - `reports/insights/REPORT_DASHBOARD.html`
   - `reports/insights/28_export_plan.md`
   - `reports/insights/27_archive_plan.md`
   - `reports/insights/06_security_scan.json`
8. Repeat once with Incremental Export enabled to verify `29_export_comparison_report.md`.

## Rollback

This is a local application. To rollback, keep the previous ZIP/source folder or reinstall the previous EXE. User settings live in the home folder and can be reset from the UI.
