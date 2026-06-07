from __future__ import annotations

import re
from collections import Counter
from pathlib import Path
from typing import Any

from ...utils.inventory import extension_key, iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, should_consider_text_file
from ...utils.time_utils import human_now
from .dependency_graph import collect_dependency_graph

_IMPORT_RE = re.compile(r"\b(import|from|require\s*\(|include|using)\b")
_CLASS_FUNC_RE = re.compile(r"\b(class|def|function|const|let|var|interface|type|struct|func)\b")
_ENTRYPOINT_NAMES = {"main.py", "__main__.py", "app.py", "server.py", "manage.py", "main.tsx", "main.ts", "index.tsx", "index.ts", "main.go"}
_CONFIG_NAMES = {"package.json", "pyproject.toml", "go.mod", "Cargo.toml", "docker-compose.yml", "docker-compose.yaml", "Dockerfile", "README.md"}


def _score_file(path: Path, root: Path, imported_by: Counter[Path], max_bytes_per_file: int) -> tuple[int, list[str]]:
    score = 0
    reasons: list[str] = []
    name = path.name
    rel_parts = [part.lower() for part in path.relative_to(root).parts]
    suffix = extension_key(path)

    if name in _ENTRYPOINT_NAMES:
        score += 40
        reasons.append("entrypoint/bootstrap filename")
    if name in _CONFIG_NAMES or name.lower().startswith(("vite.config", "next.config", "eslint.config", "tailwind.config")):
        score += 25
        reasons.append("important configuration/build file")
    if imported_by[path]:
        boost = min(40, imported_by[path] * 8)
        score += boost
        reasons.append(f"imported by {imported_by[path]} internal file(s)")
    if any(part in {"services", "api", "routes", "controllers", "stores", "domain", "core", "ui"} for part in rel_parts):
        score += 12
        reasons.append("located in an architecturally important folder")

    if should_consider_text_file(path):
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is not None:
            line_count = len(text.splitlines())
            if line_count >= 300:
                score += 18
                reasons.append(f"large file ({line_count:,} lines)")
            elif line_count >= 120:
                score += 10
                reasons.append(f"medium-size implementation file ({line_count:,} lines)")
            import_count = len(_IMPORT_RE.findall(text))
            symbol_count = len(_CLASS_FUNC_RE.findall(text))
            if import_count >= 8:
                score += 10
                reasons.append(f"many imports/references ({import_count})")
            if symbol_count >= 10:
                score += 10
                reasons.append(f"many declarations ({symbol_count})")
    if suffix in {"py", "ts", "tsx", "js", "jsx", "go", "rs"}:
        score += 5
    return score, reasons


def write_key_files_report(copied_root: Path, output_file: Path, inventory: dict[str, Any], max_bytes_per_file: int) -> None:
    graph = collect_dependency_graph(copied_root, max_bytes_per_file=max_bytes_per_file)
    imported_by: Counter[Path] = Counter()
    for targets in graph.values():
        for target in targets:
            imported_by[target] += 1

    scored: list[tuple[int, Path, list[str]]] = []
    for path in iter_project_files(copied_root):
        score, reasons = _score_file(path, copied_root, imported_by, max_bytes_per_file)
        if score > 0:
            scored.append((score, path, reasons))
    scored.sort(key=lambda item: (-item[0], rel_display(item[1], copied_root).lower()))

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# Key Files Report: {copied_root.name}\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write("This report ranks files by likely importance: entrypoints, configuration files, central imports, size, and architectural location.\n\n")
        if scored:
            for score, path, reasons in scored[:80]:
                out.write(f"## `{rel_display(path, copied_root)}`\n\n")
                out.write(f"Score: **{score}**\n\n")
                for reason in reasons:
                    out.write(f"- {reason}\n")
                out.write("\n")
        else:
            out.write("No key files were identified.\n")
