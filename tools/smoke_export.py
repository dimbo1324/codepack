"""Minimal export smoke script.

Calls ProjectExporter directly (no GUI) on a tiny sample project.
Run: .venv\\Scripts\\python.exe tools\\smoke_export.py
Output goes to .tmp\\smoke_output\\ to stay inside the project (ignored by .gitignore).
"""

from __future__ import annotations

import io
import sys

if hasattr(sys.stdout, "buffer"):
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "buffer"):
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")
import threading
from pathlib import Path
from queue import Queue

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"
if str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))

from project_exporter_desktop.config import Config
from project_exporter_desktop.models import ExportPaths
from project_exporter_desktop.services.exporter import ProjectExporter
from project_exporter_desktop.utils.time_utils import now_stamp


def _make_export_paths(source_root: Path, output_dir: Path) -> ExportPaths:
    """Return ExportPaths rooted in output_dir instead of Desktop."""
    from project_exporter_desktop.utils.path_utils import sanitize_name

    project_name = sanitize_name(source_root.name)
    stamp = now_stamp()
    bundle_name = f"{project_name}_export_{stamp}"
    staging = output_dir / bundle_name
    final_zip = output_dir / f"{bundle_name}.zip"
    archive_set_dir = output_dir / f"{bundle_name}_archives"
    reports_dir = staging / "reports"
    insights_dir = reports_dir / "insights"
    return ExportPaths(
        desktop=output_dir,
        source_root=source_root,
        project_name=project_name,
        bundle_name=bundle_name,
        staging_dir=staging,
        final_zip=final_zip,
        archive_set_dir=archive_set_dir,
        project_dir=staging / project_name,
        reports_dir=reports_dir,
        insights_dir=insights_dir,
        manifest_file=staging / "manifest.json",
        project_profile_file=staging / "PROJECT_PROFILE.json",
        index_file=staging / "INDEX.md",
        structure_report=reports_dir / "01_structure.txt",
        git_report=reports_dir / "02_git.txt",
        text_dump=reports_dir / "03_text_dump.txt",
    )


def run_smoke() -> int:
    sample = ROOT / ".tmp" / "packaging_smoke_sample"
    output_dir = ROOT / ".tmp" / "smoke_output"
    output_dir.mkdir(parents=True, exist_ok=True)

    if not sample.exists():
        print(f"ERROR: sample project not found at {sample}", file=sys.stderr)
        return 1

    cfg = Config(
        safe_export_mode="safe",
        export_profile="minimal",
        include_project_in_zip=True,
        keep_staging_folder=False,
        redact_secrets=True,
        text_file_size_limit_enabled=True,
        max_text_file_mb=1,
    )

    log_queue: Queue[str] = Queue()
    cancel = threading.Event()

    import project_exporter_desktop.services.exporter as exporter_mod
    import project_exporter_desktop.utils.path_utils as pu_mod

    original_build = pu_mod.build_export_paths

    def patched_build(source_root: Path) -> ExportPaths:
        return _make_export_paths(source_root, output_dir)

    exporter_mod.build_export_paths = patched_build

    try:
        exporter = ProjectExporter(
            source_root=sample,
            config=cfg,
            log_queue=log_queue,
            cancel_event=cancel,
        )
        paths = exporter.run()
    finally:
        exporter_mod.build_export_paths = original_build

    messages = []
    while not log_queue.empty():
        messages.append(log_queue.get_nowait())

    print("\n=== Export smoke log ===")
    for msg in messages:
        print(" ", msg)

    print("\n=== Results ===")
    print(f"  Staging dir : {paths.staging_dir}")
    print(f"  Final ZIP   : {paths.final_zip}")
    print(f"  ZIP exists  : {paths.final_zip.exists()}")

    if paths.final_zip.exists():
        size_kb = paths.final_zip.stat().st_size // 1024
        print(f"  ZIP size    : {size_kb} KB")

    import zipfile as zf

    zip_ok = paths.final_zip.exists()
    env_excluded = True
    nm_excluded = True
    main_included = False

    if zip_ok:
        with zf.ZipFile(paths.final_zip) as z:
            names = z.namelist()
            env_excluded = not any(".env" in n and not n.endswith(".example") for n in names)
            nm_excluded = not any("node_modules" in n for n in names)
            main_included = any(n.endswith("main.py") for n in names)

    print(f"\n  ZIP created             : {zip_ok} (expected: True)")
    print(f"  .env excluded from ZIP  : {env_excluded} (expected: True)")
    print(f"  node_modules excluded   : {nm_excluded} (expected: True)")
    print(f"  src/main.py in ZIP      : {main_included} (expected: True)")

    success = zip_ok and env_excluded and nm_excluded and main_included
    print(f"\n  Smoke result: {'PASS' if success else 'FAIL'}")
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(run_smoke())