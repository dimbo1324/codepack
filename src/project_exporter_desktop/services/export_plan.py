from __future__ import annotations

import json
import os
from collections import Counter
from dataclasses import asdict, dataclass, field
from pathlib import Path

from ..config import Config
from ..utils.path_utils import rel_display, should_ignore_dir
from ..utils.text_utils import format_bytes
from ..utils.time_utils import human_now
from .export_ignore import ExportIgnoreRules
from .export_policy import should_skip_file_for_safety
from .git_diff import DiffSelection
from .incremental import IncrementalSelection


@dataclass(slots=True)
class PlannedFile:
    relative_path: str
    size: int
    status: str
    reason: str = ""
    severity: str = "info"
    group: str = "other"


@dataclass(slots=True)
class ExportPlan:
    generated_at: str
    project_name: str
    source_root: str
    profile: str
    safe_export_mode: str
    diff_export_mode: str
    incremental_enabled: bool
    included_files: list[PlannedFile] = field(default_factory=list)
    excluded_files: list[PlannedFile] = field(default_factory=list)
    skipped_dirs: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    rules: dict[str, object] = field(default_factory=dict)

    @property
    def included_count(self) -> int:
        return len(self.included_files)

    @property
    def excluded_count(self) -> int:
        return len(self.excluded_files)

    @property
    def estimated_included_bytes(self) -> int:
        return sum(item.size for item in self.included_files)

    @property
    def sensitive_warnings(self) -> list[PlannedFile]:
        return [item for item in self.excluded_files if item.severity in {"critical", "high"}]

    @property
    def large_files(self) -> list[PlannedFile]:
        return [
            item
            for item in self.included_files + self.excluded_files
            if item.size >= 100 * 1024 * 1024
        ]


def _normalise_rel(path: Path | str) -> str:
    return str(path).replace("/", "\\")


def _classify_group(path: Path) -> str:
    name = path.name.casefold()
    suffix = path.suffix.casefold().lstrip(".")
    parts = [part.casefold() for part in path.parts]
    if any(part in {"test", "tests", "__tests__"} for part in parts) or name.startswith("test_"):
        return "tests"
    if suffix in {"py", "pyw", "pyi"}:
        return "python_source"
    if suffix in {"js", "jsx", "ts", "tsx", "css", "scss", "html", "vue", "svelte"}:
        return "frontend_source"
    if suffix in {"go", "rs", "java", "kt", "cs", "c", "cpp", "h", "hpp"}:
        return "backend_or_system_source"
    if suffix in {"md", "rst", "adoc", "txt"} or name in {"readme.md", "license"}:
        return "docs"
    if suffix in {"json", "yaml", "yml", "toml", "ini", "cfg", "conf", "lock"} or name.startswith(
        "dockerfile"
    ):
        return "config_and_locks"
    if suffix in {"png", "jpg", "jpeg", "webp", "gif", "svg", "ico", "pdf", "docx", "xlsx", "pptx"}:
        return "assets_and_binary_docs"
    if suffix in {"db", "sqlite", "sqlite3", "csv", "tsv", "sql", "dump", "bak"}:
        return "data_and_exports"
    return "other"


def _selected_by_diff_and_incremental(
    rel_key: str, diff_selection: DiffSelection, incremental: IncrementalSelection
) -> bool:
    selected_sets: list[frozenset[str]] = []
    if diff_selection.paths is not None:
        selected_sets.append(diff_selection.paths)
    if incremental.paths is not None:
        selected_sets.append(incremental.paths)
    if not selected_sets:
        return True
    return all(rel_key in selected for selected in selected_sets)


def build_export_plan(
    source_root: Path,
    config: Config,
    ignored_dirs: frozenset[str] | set[str],
    diff_selection: DiffSelection,
    incremental_selection: IncrementalSelection,
    export_rules: ExportIgnoreRules,
) -> ExportPlan:
    plan = ExportPlan(
        generated_at=human_now(),
        project_name=source_root.name,
        source_root=str(source_root),
        profile=config.normalized_export_profile(),
        safe_export_mode=config.normalized_safe_export_mode(),
        diff_export_mode=config.normalized_diff_export_mode(),
        incremental_enabled=incremental_selection.enabled,
        rules=export_rules.to_dict(),
    )
    if diff_selection.warning:
        plan.warnings.append(diff_selection.warning)
    if incremental_selection.warning:
        plan.warnings.append(incremental_selection.warning)

    for current_dir, dirnames, filenames in os.walk(source_root, topdown=True, followlinks=False):
        current = Path(current_dir)
        safe_dirnames: list[str] = []
        for dirname in dirnames:
            child = current / dirname
            try:
                rel_dir = child.relative_to(source_root)
            except ValueError:
                rel_dir = Path(dirname)
            if should_ignore_dir(dirname, ignored_dirs) or child.is_symlink():
                plan.skipped_dirs.append(rel_display(child, source_root))
                continue
            skip_by_rule, reason = export_rules.should_skip_dir(rel_dir)
            if skip_by_rule:
                plan.skipped_dirs.append(f"{rel_display(child, source_root)} ({reason})")
                continue
            safe_dirnames.append(dirname)
        dirnames[:] = safe_dirnames

        for filename in filenames:
            path = current / filename
            if path.is_symlink():
                continue
            try:
                relative_path = path.relative_to(source_root)
            except ValueError:
                continue
            rel_key = _normalise_rel(relative_path)
            try:
                size = path.stat().st_size
            except Exception:
                size = 0
            group = _classify_group(relative_path)

            if not _selected_by_diff_and_incremental(
                rel_key, diff_selection, incremental_selection
            ):
                plan.excluded_files.append(
                    PlannedFile(
                        rel_key,
                        size,
                        "excluded",
                        "not selected by diff/incremental mode",
                        "info",
                        group,
                    )
                )
                continue
            skip_by_rule, reason = export_rules.should_skip_file(relative_path)
            if skip_by_rule:
                plan.excluded_files.append(
                    PlannedFile(rel_key, size, "excluded", reason, "medium", group)
                )
                continue
            safety = should_skip_file_for_safety(
                relative_path, config.normalized_safe_export_mode()
            )
            if safety.skip:
                plan.excluded_files.append(
                    PlannedFile(rel_key, size, "excluded", safety.reason, safety.severity, group)
                )
                continue
            plan.included_files.append(PlannedFile(rel_key, size, "included", "", "info", group))
    return plan


def write_export_plan_files(plan: ExportPlan, output_json: Path, output_md: Path) -> None:
    output_json.parent.mkdir(parents=True, exist_ok=True)
    data = asdict(plan)
    data["summary"] = {
        "included_count": plan.included_count,
        "excluded_count": plan.excluded_count,
        "estimated_included_bytes": plan.estimated_included_bytes,
        "estimated_included_size": format_bytes(plan.estimated_included_bytes),
        "skipped_dirs_count": len(plan.skipped_dirs),
    }
    output_json.write_text(
        json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8", newline="\n"
    )

    by_group = Counter(item.group for item in plan.included_files)
    with output_md.open("w", encoding="utf-8", newline="\n") as out:
        out.write("# Export Plan\n\n")
        out.write(f"Generated: {plan.generated_at}\n\n")
        out.write(f"- Project: `{plan.project_name}`\n")
        out.write(f"- Profile: `{plan.profile}`\n")
        out.write(f"- Safe mode: `{plan.safe_export_mode}`\n")
        out.write(f"- Diff mode: `{plan.diff_export_mode}`\n")
        out.write(f"- Incremental: `{plan.incremental_enabled}`\n")
        out.write(f"- Included files: {plan.included_count:,}\n")
        out.write(f"- Excluded files: {plan.excluded_count:,}\n")
        out.write(f"- Skipped directories: {len(plan.skipped_dirs):,}\n")
        out.write(
            f"- Estimated selected project size: {format_bytes(plan.estimated_included_bytes)}\n\n"
        )
        if plan.warnings:
            out.write("## Warnings\n\n")
            for warning in plan.warnings:
                out.write(f"- {warning}\n")
            out.write("\n")
        out.write("## Included files by group\n\n")
        for group, count in by_group.most_common():
            out.write(f"- {group}: {count:,}\n")
        out.write("\n## Sensitive / high-risk excluded files\n\n")
        sensitive = plan.sensitive_warnings[:200]
        if not sensitive:
            out.write("- none\n")
        else:
            for item in sensitive:
                out.write(
                    f"- [{item.severity}] `{item.relative_path}` — {item.reason} ({format_bytes(item.size)})\n"
                )
        out.write("\n## Large files\n\n")
        large = sorted(plan.large_files, key=lambda item: item.size, reverse=True)[:100]
        if not large:
            out.write("- none >= 100 MB\n")
        else:
            for item in large:
                out.write(
                    f"- `{item.relative_path}` — {format_bytes(item.size)} — {item.status} — {item.reason or item.group}\n"
                )
        out.write("\n## First included files\n\n")
        for item in plan.included_files[:300]:
            out.write(f"- `{item.relative_path}` ({format_bytes(item.size)})\n")
        if plan.included_count > 300:
            out.write(f"- ... and {plan.included_count - 300:,} more\n")


def format_export_plan_for_user(plan: ExportPlan, zip_limit_bytes: int) -> str:
    lines = [
        "Export Plan",
        "",
        f"Project: {plan.project_name}",
        f"Profile: {plan.profile}",
        f"Safe mode: {plan.safe_export_mode}",
        f"Diff mode: {plan.diff_export_mode}",
        f"Incremental: {'yes' if plan.incremental_enabled else 'no'}",
        f"Included files: {plan.included_count:,}",
        f"Excluded files: {plan.excluded_count:,}",
        f"Skipped folders: {len(plan.skipped_dirs):,}",
        f"Estimated selected project size: {format_bytes(plan.estimated_included_bytes)}",
        f"ZIP part limit: {format_bytes(zip_limit_bytes)}",
        "",
        "Top warnings:",
    ]
    if plan.warnings:
        lines.extend(f"- {warning}" for warning in plan.warnings[:5])
    else:
        lines.append("- none")
    lines.extend(["", "Sensitive/high-risk excluded files:"])
    sensitive = plan.sensitive_warnings[:20]
    if sensitive:
        lines.extend(
            f"- [{item.severity}] {item.relative_path}: {item.reason}" for item in sensitive
        )
    else:
        lines.append("- none")
    lines.extend(["", "Largest files in selection/exclusions:"])
    large = sorted(plan.large_files, key=lambda item: item.size, reverse=True)[:10]
    if large:
        lines.extend(
            f"- {item.relative_path}: {format_bytes(item.size)} ({item.status})" for item in large
        )
    else:
        lines.append("- none >= 100 MB")
    lines.extend(["", "Continue export?"])
    return "\n".join(lines)
