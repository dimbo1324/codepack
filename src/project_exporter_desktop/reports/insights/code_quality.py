from __future__ import annotations

import ast
import re
from collections import Counter, defaultdict
from pathlib import Path

from ...constants import SOURCE_CODE_EXTENSIONS, TODO_PATTERN
from ...utils.inventory import extension_key, iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, should_consider_text_file
from ...utils.time_utils import human_now

_DANGEROUS_MIX_RE = re.compile(
    r"\b(tkinter|subprocess|requests|fetch|sql|database|threading|Queue|open\(|write_text|read_text)\b"
)


def _python_function_lengths(path: Path) -> list[tuple[str, int, int]]:
    try:
        tree = ast.parse(path.read_text(encoding="utf-8", errors="replace"))
    except Exception:
        return []
    results: list[tuple[str, int, int]] = []
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            end = getattr(node, "end_lineno", None)
            if end and node.lineno:
                results.append((node.name, node.lineno, end - node.lineno + 1))
    return results


def write_code_quality_report(
    copied_root: Path, output_file: Path, max_bytes_per_file: int | None
) -> None:
    large_files: list[tuple[Path, int]] = []
    long_symbols: list[tuple[Path, str, int, int]] = []
    todo_files: Counter[Path] = Counter()
    duplicate_names: dict[str, list[Path]] = defaultdict(list)
    mixed_responsibility: list[tuple[Path, list[str]]] = []
    source_files = []

    for path in iter_project_files(copied_root):
        duplicate_names[path.name.lower()].append(path)
        ext = extension_key(path)
        if ext not in SOURCE_CODE_EXTENSIONS:
            continue
        source_files.append(path)
        if not should_consider_text_file(path):
            continue
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue
        line_count = len(text.splitlines())
        if line_count >= 400:
            large_files.append((path, line_count))
        for _match in TODO_PATTERN.finditer(text):
            todo_files[path] += 1
        if ext == "py":
            for name, line, length in _python_function_lengths(path):
                if length >= 80:
                    long_symbols.append((path, name, line, length))
        signals: list[str] = []
        lower_path = str(path.relative_to(copied_root)).lower()
        if "ui" in lower_path and re.search(
            r"\b(shutil|zipfile|subprocess|os\.walk|threading)\b", text
        ):
            signals.append("UI file appears to contain infrastructure/threading/file-system logic")
        if len(set(_DANGEROUS_MIX_RE.findall(text))) >= 4 and line_count >= 180:
            signals.append("many mixed technical concerns in a medium/large file")
        if signals:
            mixed_responsibility.append((path, signals))

    duplicate_groups = {
        name: paths
        for name, paths in duplicate_names.items()
        if len(paths) >= 3 and name not in {"index.ts", "index.tsx", "__init__.py"}
    }

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# Code Quality Report\n\nGenerated: {human_now()}\n\n")
        out.write(
            "This report highlights maintainability risks using static heuristics. Treat findings as review prompts, not absolute errors.\n\n"
        )
        out.write("## Summary\n\n")
        out.write(f"- Source files analysed: {len(source_files):,}\n")
        out.write(f"- Large files >= 400 lines: {len(large_files):,}\n")
        out.write(f"- Long Python classes/functions >= 80 lines: {len(long_symbols):,}\n")
        out.write(f"- Files with TODO/FIXME-like markers: {len(todo_files):,}\n")
        out.write(f"- Duplicate filename groups: {len(duplicate_groups):,}\n\n")

        out.write("## Large files\n\n")
        if large_files:
            for path, lines in sorted(large_files, key=lambda item: item[1], reverse=True)[:100]:
                out.write(f"- `{rel_display(path, copied_root)}` — {lines:,} lines\n")
        else:
            out.write("No large source files detected.\n")

        out.write("\n## Long Python classes/functions\n\n")
        if long_symbols:
            for path, name, line, length in sorted(
                long_symbols, key=lambda item: item[3], reverse=True
            )[:100]:
                out.write(
                    f"- `{rel_display(path, copied_root)}`:{line} `{name}` — {length:,} lines\n"
                )
        else:
            out.write("No long Python symbols detected.\n")

        out.write("\n## Possible mixed-responsibility files\n\n")
        if mixed_responsibility:
            for path, signals in mixed_responsibility[:100]:
                out.write(f"- `{rel_display(path, copied_root)}`\n")
                for signal in signals:
                    out.write(f"  - {signal}\n")
        else:
            out.write("No obvious mixed-responsibility files detected.\n")

        out.write("\n## Repeated filenames\n\n")
        if duplicate_groups:
            for name, paths in sorted(
                duplicate_groups.items(), key=lambda item: len(item[1]), reverse=True
            )[:50]:
                out.write(f"### `{name}` ({len(paths)} files)\n")
                for path in sorted(paths, key=lambda p: str(p).lower())[:20]:
                    out.write(f"- `{rel_display(path, copied_root)}`\n")
                out.write("\n")
        else:
            out.write("No concerning duplicate filename groups detected.\n")
