from __future__ import annotations

import re
from pathlib import Path
from typing import Any

from ...constants import SOURCE_CODE_EXTENSIONS
from ...utils.inventory import extension_key, iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, should_consider_text_file
from ...utils.time_utils import human_now
from .dependency_graph import collect_dependency_graph


def write_refactoring_opportunities_report(
    copied_root: Path,
    output_file: Path,
    inventory: dict[str, Any],
    max_bytes_per_file: int,
) -> None:
    graph = collect_dependency_graph(copied_root, max_bytes_per_file=max_bytes_per_file)
    imported_by: dict[Path, int] = {path: 0 for path in graph}
    for targets in graph.values():
        for target in targets:
            imported_by[target] = imported_by.get(target, 0) + 1

    opportunities: list[tuple[int, Path, list[str], list[str]]] = []
    for path in iter_project_files(copied_root):
        ext = extension_key(path)
        if ext not in SOURCE_CODE_EXTENSIONS or not should_consider_text_file(path):
            continue
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue
        lines = text.splitlines()
        reasons: list[str] = []
        suggestions: list[str] = []
        score = 0
        line_count = len(lines)
        if line_count >= 500:
            score += 40
            reasons.append(f"very large file ({line_count:,} lines)")
            suggestions.append("split by responsibility into smaller modules/components")
        elif line_count >= 300:
            score += 25
            reasons.append(f"large file ({line_count:,} lines)")
            suggestions.append("review whether UI, orchestration, and pure helpers can be separated")
        inbound = imported_by.get(path, 0)
        outbound = len(graph.get(path, set()))
        if inbound >= 5:
            score += min(35, inbound * 5)
            reasons.append(f"central dependency imported by {inbound} internal file(s)")
            suggestions.append("keep public functions stable; extract volatile logic behind small interfaces")
        if outbound >= 10:
            score += 20
            reasons.append(f"high outgoing dependency count ({outbound})")
            suggestions.append("group related dependencies behind service/helper modules")
        if re.search(r"\bTODO|FIXME|HACK|XXX|DEPRECATED\b", text, re.IGNORECASE):
            score += 8
            reasons.append("contains technical-debt markers")
            suggestions.append("convert repeated TODO/FIXME items into tracked tasks")
        lower_rel = str(path.relative_to(copied_root)).lower()
        if "ui" in lower_rel and re.search(r"\b(shutil|zipfile|subprocess|os\.walk|threading|Queue)\b", text):
            score += 25
            reasons.append("UI code contains infrastructure or worker orchestration concerns")
            suggestions.append("move long-running work and file-system operations behind service/worker classes")
        if score:
            opportunities.append((score, path, reasons, sorted(set(suggestions))))

    opportunities.sort(key=lambda item: (-item[0], rel_display(item[1], copied_root).lower()))

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("# Refactoring Opportunities\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write("This report translates metrics and dependency signals into practical refactoring candidates.\n\n")
        if opportunities:
            for score, path, reasons, suggestions in opportunities[:80]:
                out.write(f"## `{rel_display(path, copied_root)}`\n\n")
                out.write(f"Priority score: **{score}**\n\n")
                out.write("Reasons:\n")
                for reason in reasons:
                    out.write(f"- {reason}\n")
                out.write("\nSuggested actions:\n")
                for suggestion in suggestions:
                    out.write(f"- {suggestion}\n")
                out.write("\n")
        else:
            out.write("No significant refactoring opportunities detected by the current heuristic rules.\n")
