from __future__ import annotations

import json
from dataclasses import asdict
from typing import Any

from ..config import Config
from ..constants import APP_NAME, APP_VERSION, IGNORED_DIR_NAMES, REPORT_DESCRIPTIONS
from ..models import ArchiveBuildResult, CopyStats, ExportPaths, TextDumpStats
from ..services.diff_service import DiffSelection, diff_manifest_payload
from ..utils.text_utils import format_bytes
from ..utils.time_utils import human_now


def _archive_payload(archive_result: ArchiveBuildResult | None) -> dict[str, Any]:
    if archive_result is None:
        return {"status": "not_written_yet"}
    return {
        "split": archive_result.split,
        "output_dir": str(archive_result.output_dir) if archive_result.output_dir else None,
        "archives": [str(path) for path in archive_result.archives],
        "archive_names": [path.name for path in archive_result.archives],
        "file_count": archive_result.file_count,
        "skipped_project_files": archive_result.skipped_project_files,
        "oversized_files": archive_result.oversized_files,
    }


def write_manifest(
    paths: ExportPaths,
    config: Config,
    copy_stats: CopyStats,
    text_stats: TextDumpStats,
    extra_ignored_dirs: frozenset[str] | set[str],
    cancelled: bool,
    archive_result: ArchiveBuildResult | None = None,
    diff_selection: DiffSelection | None = None,
) -> None:
    """Write a machine-readable description of the bundle."""
    effective_ignored = {name.casefold() for name in IGNORED_DIR_NAMES} | set(extra_ignored_dirs)
    user_extras = set(extra_ignored_dirs) - {name.casefold() for name in IGNORED_DIR_NAMES}
    text_limit = config.effective_max_text_file_bytes()
    data = {
        "app": APP_NAME,
        "app_version": APP_VERSION,
        "generated_at": human_now(),
        "bundle_name": paths.bundle_name,
        "source_root": str(paths.source_root),
        "project_name": paths.project_name,
        "cancelled": cancelled,
        "settings": {
            "text_file_size_limit_enabled": config.text_file_size_limit_enabled,
            "max_text_file_mb": config.max_text_file_mb
            if config.text_file_size_limit_enabled
            else None,
            "max_text_file_bytes": text_limit,
            "max_text_file_human": format_bytes(text_limit)
            if text_limit is not None
            else "unlimited",
            "redact_secrets": config.redact_secrets,
            "keep_staging_folder": config.keep_staging_folder,
            "include_project_in_zip": config.include_project_in_zip,
            "export_profile": config.normalized_export_profile(),
            "safe_export_mode": config.normalized_safe_export_mode(),
            "zip_part_limit_mb": config.zip_part_limit_mb,
            "diff_export_mode": config.normalized_diff_export_mode(),
            "diff_base_ref": config.diff_base_ref,
            "diff_target_ref": config.diff_target_ref,
            "include_git_patch": config.include_git_patch,
            "custom_excluded_files": config.custom_excluded_files,
            "custom_excluded_extensions": config.custom_excluded_extensions,
            "always_include_files": config.always_include_files,
            "always_include_dirs": config.always_include_dirs,
            "incremental_export_enabled": config.incremental_export_enabled,
            "theme": config.normalized_theme(),
            "watch_enabled": config.watch_enabled,
            "watch_clipboard_auto_update": config.watch_clipboard_auto_update,
            "prompt_goals": config.prompt_goals,
        },
        "diff_selection": diff_manifest_payload(diff_selection),
        "ignored_dirs": {
            "defaults": sorted({name.casefold() for name in IGNORED_DIR_NAMES}),
            "user_extras": sorted(user_extras),
            "effective": sorted(effective_ignored),
        },
        "notes": [
            "Git data is collected from the ORIGINAL project; the .git directory is never copied into the bundle.",
            f"Safe Export mode was {config.normalized_safe_export_mode()}; copy-time filtering may have removed secrets or local data.",
            "All other reports describe the copied project (see the project folder inside the bundle).",
            "Symlinks were skipped during copy to avoid accidental escape from the project tree.",
            "If archive splitting was needed, extract all ZIP files from the archive-set folder into the same destination.",
        ],
        "stats": {
            "copy": asdict(copy_stats),
            "text_dump": asdict(text_stats),
        },
        "archives": _archive_payload(archive_result),
        "layout": {
            "project_dir": paths.project_name + "/",
            "project_profile": "PROJECT_PROFILE.json",
            "reports": [path for path, _desc in REPORT_DESCRIPTIONS],
        },
    }

    paths.manifest_file.write_text(
        json.dumps(data, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def write_index_md(
    paths: ExportPaths,
    config: Config,
    extra_ignored_dirs: frozenset[str] | set[str],
) -> None:
    """Write a human-readable bundle table of contents."""
    ignored_effective = sorted(
        {name.casefold() for name in IGNORED_DIR_NAMES} | set(extra_ignored_dirs)
    )
    text_limit = config.effective_max_text_file_bytes()

    with paths.index_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write(f"# Export bundle: `{paths.project_name}`\n\n")
        out.write(f"_Generated by {APP_NAME} v{APP_VERSION} on {human_now()}._\n\n")
        out.write(
            "This archive bundles a project copy together with machine- and human-readable reports. "
            "Version 5 adds export planning, .exportignore, incremental export, dashboard, SARIF/JSON security outputs and pre-planned logical archives.\n\n"
        )

        out.write("## Bundle contents\n\n")
        out.write(
            f"- `{paths.project_name}/` — working copy of the project "
            f"(without `.git`, `node_modules`, user-configured extras and files filtered by Safe Export mode).\n"
        )
        out.write("- `manifest.json` — machine-readable metadata about this export.\n")
        out.write("- `PROJECT_PROFILE.json` — machine-readable project passport.\n")
        out.write("- `reports/` — generated reports and AI handoff material.\n\n")

        out.write("## Reports\n\n")
        for rel_path, description in REPORT_DESCRIPTIONS:
            out.write(f"- `{rel_path}` — {description}\n")

        out.write("\n## Settings used for this export\n\n")
        out.write(
            f"- Max text-file size: **{format_bytes(text_limit) if text_limit is not None else 'unlimited'}**\n"
        )
        out.write(f"- Secret redaction: **{'enabled' if config.redact_secrets else 'disabled'}**\n")
        out.write(f"- Safe Export mode: **{config.normalized_safe_export_mode()}**\n")
        out.write(f"- Diff Export mode: **{config.normalized_diff_export_mode()}**\n")
        out.write(
            f"- Incremental Export: **{'enabled' if config.incremental_export_enabled else 'disabled'}**\n"
        )
        out.write(f"- Archive part limit: **{config.zip_part_limit_mb} MB**\n")
        out.write(
            f"- Project included in ZIP: **{'yes' if config.include_project_in_zip else 'no (reports only)'}**\n"
        )
        out.write(
            f"- Staging folder kept on Desktop: **{'yes' if config.keep_staging_folder else 'no'}**\n"
        )
        out.write("- Ignored directories: " + ", ".join(f"`{n}`" for n in ignored_effective) + "\n")

        out.write("\n## Notes\n\n")
        out.write(
            "- Git data was collected from the **original** project. The `.git` directory itself is intentionally not part of the bundle.\n"
        )
        out.write("- Full Git patches are disabled by default because diffs can expose secrets.\n")
        out.write(
            "- Safe Export mode filters high-risk files during copy; still review the security report before sharing externally.\n"
        )
        out.write(
            "- Symbolic links were skipped during the copy to avoid accidental escape from the project tree.\n"
        )
        out.write(
            "- If the export was split, all archives are placed into a Desktop folder and include an `ARCHIVE_SET_MANIFEST.json`.\n"
        )
