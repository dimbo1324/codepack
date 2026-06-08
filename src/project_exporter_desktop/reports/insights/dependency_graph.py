from __future__ import annotations

import ast
import re
from collections import Counter, defaultdict
from pathlib import Path

from ...utils.inventory import extension_key, iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, should_consider_text_file
from ...utils.time_utils import human_now

_SOURCE_EXTS = {"py", "js", "jsx", "ts", "tsx", "mjs", "cjs", "vue", "svelte", "astro", "go"}
_JS_IMPORT_RE = re.compile(r"(?:from\s+['\"]([^'\"]+)['\"]|import\s*\(\s*['\"]([^'\"]+)['\"]\s*\)|require\s*\(\s*['\"]([^'\"]+)['\"]\s*\))")
_GO_IMPORT_RE = re.compile(r"^\s*(?:import\s+)?[\"`]([^\"`]+)[\"`]", re.MULTILINE)


def _module_name(path: Path, root: Path) -> str:
    rel = path.relative_to(root).with_suffix("")
    parts = list(rel.parts)
    if parts and parts[-1] == "__init__":
        parts = parts[:-1]
    return ".".join(parts)


def _resolve_python_import(current: Path, root: Path, module: str | None, level: int = 0) -> Path | None:
    base = current.parent
    if level:
        for _ in range(max(0, level - 1)):
            base = base.parent
        if module:
            candidate = base.joinpath(*module.split("."))
        else:
            candidate = base
    elif module:
        candidate = root.joinpath(*module.split("."))
    else:
        return None

    possible = [candidate.with_suffix(".py"), candidate / "__init__.py"]
    for path in possible:
        if path.exists() and path.is_file():
            return path
    return None


def _resolve_relative_import(current: Path, specifier: str, root: Path) -> Path | None:
    if not specifier.startswith("."):
        return None
    candidate = (current.parent / specifier).resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError:
        return None
    extensions = ["", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".vue", ".svelte", ".astro", ".json"]
    for ext in extensions:
        path = candidate if ext == "" else Path(str(candidate) + ext)
        if path.exists() and path.is_file():
            return path
    for ext in extensions[1:]:
        path = candidate / f"index{ext}"
        if path.exists() and path.is_file():
            return path
    return None


def _python_edges(path: Path, root: Path) -> set[Path]:
    try:
        tree = ast.parse(path.read_text(encoding="utf-8", errors="replace"))
    except Exception:
        return set()
    edges: set[Path] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                target = _resolve_python_import(path, root, alias.name, 0)
                if target:
                    edges.add(target)
        elif isinstance(node, ast.ImportFrom):
            target = _resolve_python_import(path, root, node.module, node.level)
            if target:
                edges.add(target)
    return edges


def collect_dependency_graph(root: Path, max_bytes_per_file: int | None = 1_000_000) -> dict[Path, set[Path]]:
    files = [path for path in iter_project_files(root) if extension_key(path) in _SOURCE_EXTS]
    graph: dict[Path, set[Path]] = {path: set() for path in files}

    for path in files:
        ext = extension_key(path)
        if ext == "py":
            graph[path].update(_python_edges(path, root))
            continue
        if not should_consider_text_file(path):
            continue
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue
        if ext in {"js", "jsx", "ts", "tsx", "mjs", "cjs", "vue", "svelte", "astro"}:
            for match in _JS_IMPORT_RE.finditer(text):
                spec = next((group for group in match.groups() if group), "")
                target = _resolve_relative_import(path, spec, root)
                if target:
                    graph[path].add(target)
        elif ext == "go":
            module_name = ""
            go_mod = root / "go.mod"
            if go_mod.exists():
                for line in go_mod.read_text(encoding="utf-8", errors="replace").splitlines():
                    if line.startswith("module "):
                        module_name = line.split(maxsplit=1)[1].strip()
                        break
            for spec in _GO_IMPORT_RE.findall(text):
                if module_name and spec.startswith(module_name):
                    rel = spec.removeprefix(module_name).lstrip("/")
                    candidate = root / rel
                    if candidate.exists() and candidate.is_dir():
                        for go_file in candidate.glob("*.go"):
                            graph[path].add(go_file)
                            break
    return {path: {target for target in targets if target.exists()} for path, targets in graph.items()}


def _node_id(path: Path, root: Path) -> str:
    rel = str(path.relative_to(root)).replace("\\", "/")
    return re.sub(r"[^A-Za-z0-9_]", "_", rel)


def write_dependency_graph_reports(copied_root: Path, output_file: Path, mermaid_file: Path, max_bytes_per_file: int | None) -> None:
    graph = collect_dependency_graph(copied_root, max_bytes_per_file=max_bytes_per_file)
    in_degree: Counter[Path] = Counter()
    for targets in graph.values():
        for target in targets:
            in_degree[target] += 1
    edge_count = sum(len(targets) for targets in graph.values())
    top_imported = in_degree.most_common(30)

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# Dependency Graph\n\nGenerated: {human_now()}\n\n")
        out.write("This report maps internal imports/references using static heuristics. It is intentionally dependency-free and may not resolve every alias.\n\n")
        out.write(f"- Files in graph: **{len(graph):,}**\n")
        out.write(f"- Internal edges: **{edge_count:,}**\n\n")

        out.write("## Most imported internal files\n\n")
        if top_imported:
            for path, count in top_imported:
                out.write(f"- `{rel_display(path, copied_root)}` — imported by {count:,} file(s)\n")
        else:
            out.write("No internal imports were resolved.\n")

        out.write("\n## Internal import edges\n\n")
        emitted = 0
        for source, targets in sorted(graph.items(), key=lambda item: rel_display(item[0], copied_root).lower()):
            if not targets:
                continue
            out.write(f"### `{rel_display(source, copied_root)}`\n\n")
            for target in sorted(targets, key=lambda p: rel_display(p, copied_root).lower()):
                out.write(f"- `{rel_display(target, copied_root)}`\n")
                emitted += 1
                if emitted >= 1000:
                    out.write("\n_Output truncated after 1,000 edges._\n")
                    break
            out.write("\n")
            if emitted >= 1000:
                break

    with mermaid_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("graph TD\n")
        emitted = 0
        for source, targets in sorted(graph.items(), key=lambda item: rel_display(item[0], copied_root).lower()):
            for target in sorted(targets, key=lambda p: rel_display(p, copied_root).lower()):
                out.write(f"  {_node_id(source, copied_root)}[\"{rel_display(source, copied_root)}\"] --> {_node_id(target, copied_root)}[\"{rel_display(target, copied_root)}\"]\n")
                emitted += 1
                if emitted >= 250:
                    out.write("  %% Mermaid output truncated after 250 edges.\n")
                    return
        if emitted == 0:
            out.write(f"  root[\"{copied_root.name}\"]\n")
