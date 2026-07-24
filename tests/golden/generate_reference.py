"""Generates golden reference artifacts by running the legacy Python implementation.

This script is the *only* thing in the repository that executes the archived legacy
version (`docs/__arch__/codepack-main.zip`). It is invoked by `cargo xtask golden`
on a developer machine and never by CI: the artifacts it produces are committed, so
the Rust test suite compares against files, not against a live Python run.

Why this exists at all: the S2 and S9 acceptance criteria require the export to match
the legacy implementation's own output. Until 2026-07-25 that was checked by *reading*
the archived Python sources, which verifies intent rather than result. Running legacy
turned out to cost seconds (`exporter.py` imports no PySide6 and needs no installed
dependencies), and the very first run found a real invariant I5 break, so the
criteria are now met by execution.

## The comparison specification

This spec is implemented twice on purpose -- here in Python (when writing references)
and in Rust (when comparing our own output). They are two implementations of one
written contract, not accidental duplication: if they ever disagree, the golden test
fails, which is exactly the signal we want.

Volatile keys, dropped anywhere they appear in a JSON tree:

    generated_at, source_root, copied_root, bundle_name

Per-artifact rules:

* `28_export_plan.json`, `06_security_scan.json`, `PROJECT_PROFILE.json` -- compared in
  full after dropping volatile keys. Every field in these describes the *source
  project*, so any difference is a real difference.
* `REPORT_PLUGINS.json` -- compared as a **superset with identical gating**: every
  entry legacy lists must be present with exactly the same `profiles` set, in legacy's
  own filename order. Two differences are intended and therefore not compared:
    - `description`: legacy declares the field but leaves it empty for every entry.
      Ours fills it. BLUEPRINT calls this catalog the seed of a public report-plugin
      API (§A/§B.10), and an empty description field serves nobody; filling a field
      that already exists breaks no consumer.
    - extra entries (`AI_CONTEXT`, `AI_PROMPTS`, `REPORT_DASHBOARD.html`): legacy
      produces these artifacts but builds them outside its `report_jobs` list, so its
      catalog silently omits them. Ours models them as real jobs, so omitting them from
      the catalog would make the catalog lie about our own pipeline.
  A *missing* legacy entry, or a changed `profiles` set, still fails -- that is where
  gating regressions actually hide.
* `manifest.json` -- only the source-describing subtrees are compared: `project_name`,
  `cancelled`, `settings`, `diff_selection`, `ignored_dirs`. Deliberately excluded and
  why:
    - `app`/`app_version`: we are `codepack`, legacy was `Project Exporter Desktop`.
      An intended rebrand (decision Q5, 2026-07-25), not drift.
    - `stats`/`archives`/`layout`/`notes`: these measure the export's *own output*,
      whose prose legitimately differs from legacy's report-for-report text.
* `27_archive_plan.json` -- only the pure-configuration fields (`split_planned`,
  `limit_bytes`, `target_bytes`); every other number in it measures our own bundle.
* `artifact_names.json` -- the sorted list of entry names in the produced ZIP, with the
  copied-project subdirectory renamed to a fixed placeholder. Compared in full: a
  missing or extra artifact is precisely the class of regression this catches.
"""

from __future__ import annotations

import json
import pathlib
import shutil
import sys
import threading
import zipfile
from queue import Queue

VOLATILE_KEYS = frozenset({"generated_at", "source_root", "copied_root", "bundle_name"})

FULL_COMPARE_ARTIFACTS = {
    "28_export_plan.json": "reports/insights/28_export_plan.json",
    "06_security_scan.json": "reports/insights/06_security_scan.json",
    "PROJECT_PROFILE.json": "PROJECT_PROFILE.json",
    "REPORT_PLUGINS.json": "reports/insights/REPORT_PLUGINS.json",
}

MANIFEST_COMPARED_KEYS = (
    "project_name",
    "cancelled",
    "settings",
    "diff_selection",
    "ignored_dirs",
)

ARCHIVE_PLAN_COMPARED_KEYS = ("split_planned", "limit_bytes", "target_bytes")

PROJECT_DIR_PLACEHOLDER = "<PROJECT>"


def strip_volatile(value):
    """Recursively drops every volatile key. Mirrored by the Rust side's own stripper."""
    if isinstance(value, dict):
        return {
            key: strip_volatile(item)
            for key, item in value.items()
            if key not in VOLATILE_KEYS
        }
    if isinstance(value, list):
        return [strip_volatile(item) for item in value]
    return value


def run_legacy(legacy_src: pathlib.Path, fixture: pathlib.Path, out_root: pathlib.Path):
    """Runs one legacy export and returns the path of the ZIP it produced."""
    sys.path.insert(0, str(legacy_src))

    from project_exporter_desktop.utils import path_utils

    path_utils.desktop_path = lambda: out_root

    from project_exporter_desktop.config import Config
    from project_exporter_desktop.services.exporter import ProjectExporter

    paths = ProjectExporter(fixture, Config(), Queue(), threading.Event()).run()
    produced = out_root / f"{paths.bundle_name}.zip"
    if not produced.exists():
        raise SystemExit(f"legacy produced no archive for {fixture.name}")
    return produced, paths.project_name


def artifact_names(archive: zipfile.ZipFile, project_name: str) -> list[str]:
    """ZIP entry names with the copied-project subdirectory replaced by a placeholder.

    The project directory is named after the fixture, so without this the listing would
    encode the fixture name into every source-file entry and compare nothing useful.
    """
    names = []
    for name in archive.namelist():
        normalized = name.replace("\\", "/")
        if normalized.startswith(f"{project_name}/"):
            normalized = PROJECT_DIR_PLACEHOLDER + normalized[len(project_name) :]
        names.append(normalized)
    return sorted(names)


def extract_reference(archive_path: pathlib.Path, project_name: str, dest: pathlib.Path):
    dest.mkdir(parents=True, exist_ok=True)
    written = []

    with zipfile.ZipFile(archive_path) as archive:
        entries = set(archive.namelist())

        for out_name, entry in FULL_COMPARE_ARTIFACTS.items():
            if entry not in entries:
                raise SystemExit(f"legacy did not produce {entry}")
            payload = strip_volatile(json.loads(archive.read(entry)))
            (dest / out_name).write_text(
                json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            written.append(out_name)

        manifest = strip_volatile(json.loads(archive.read("manifest.json")))
        subset = {key: manifest[key] for key in MANIFEST_COMPARED_KEYS if key in manifest}
        (dest / "manifest.subset.json").write_text(
            json.dumps(subset, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        written.append("manifest.subset.json")

        plan = json.loads(archive.read("reports/insights/27_archive_plan.json"))
        subset = {key: plan[key] for key in ARCHIVE_PLAN_COMPARED_KEYS if key in plan}
        (dest / "27_archive_plan.subset.json").write_text(
            json.dumps(subset, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        written.append("27_archive_plan.subset.json")

        (dest / "artifact_names.json").write_text(
            json.dumps(
                artifact_names(archive, project_name),
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        written.append("artifact_names.json")

    return written


def main() -> int:
    if len(sys.argv) != 4:
        print(
            "usage: generate_reference.py <legacy-src-dir> <fixtures-dir> <reference-dir>",
            file=sys.stderr,
        )
        return 2

    legacy_src = pathlib.Path(sys.argv[1]).resolve()
    fixtures_dir = pathlib.Path(sys.argv[2]).resolve()
    reference_dir = pathlib.Path(sys.argv[3]).resolve()

    fixtures = sorted(p for p in fixtures_dir.iterdir() if p.is_dir())
    if not fixtures:
        print(f"no fixtures found under {fixtures_dir}", file=sys.stderr)
        return 1

    staging = reference_dir.parent / ".legacy-run"
    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir(parents=True)

    try:
        for fixture in fixtures:
            out_root = staging / fixture.name
            out_root.mkdir(parents=True)
            archive_path, project_name = run_legacy(legacy_src, fixture, out_root)
            written = extract_reference(
                archive_path, project_name, reference_dir / fixture.name
            )
            print(f"{fixture.name}: {len(written)} reference artifacts")
    finally:
        shutil.rmtree(staging, ignore_errors=True)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
