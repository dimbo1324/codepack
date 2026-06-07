from __future__ import annotations

import re
from pathlib import Path

from ...utils.inventory import extension_key, iter_project_dirs, iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, safe_read_json, should_consider_text_file
from ...utils.time_utils import human_now

_COMPONENT_RE = re.compile(r"\b(?:export\s+default\s+)?function\s+([A-Z][A-Za-z0-9_]*)|\bconst\s+([A-Z][A-Za-z0-9_]*)\s*=\s*(?:\(|React\.)")
_HOOK_RE = re.compile(r"\bfunction\s+(use[A-Z][A-Za-z0-9_]*)|\bconst\s+(use[A-Z][A-Za-z0-9_]*)\s*=")
_PY_CLASS_RE = re.compile(r"^class\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE)
_PY_FUNC_RE = re.compile(r"^def\s+([A-Za-z_][A-Za-z0-9_]*)", re.MULTILINE)


def write_frontend_report(copied_root: Path, output_file: Path, max_bytes_per_file: int) -> None:
    package_json = safe_read_json(copied_root / "package.json")
    deps = {}
    for section in ("dependencies", "devDependencies"):
        value = package_json.get(section) if package_json else None
        if isinstance(value, dict):
            deps.update(value)

    dirs_by_role: dict[str, list[Path]] = {"pages": [], "routes": [], "components": [], "hooks": [], "stores": [], "styles": []}
    for directory in iter_project_dirs(copied_root):
        parts = {part.lower() for part in directory.relative_to(copied_root).parts}
        for key in dirs_by_role:
            if key in parts or (key == "styles" and {"css", "styles", "style"}.intersection(parts)):
                dirs_by_role[key].append(directory)

    components: list[tuple[Path, str]] = []
    hooks: list[tuple[Path, str]] = []
    state_files: list[Path] = []
    form_files: list[Path] = []
    route_files: list[Path] = []

    for path in iter_project_files(copied_root):
        ext = extension_key(path)
        rel_lower = str(path.relative_to(copied_root)).lower().replace("\\", "/")
        if ext not in {"js", "jsx", "ts", "tsx", "vue", "svelte", "astro"}:
            continue
        if "route" in rel_lower or "router" in rel_lower or "/pages/" in rel_lower or path.name.lower().startswith("page."):
            route_files.append(path)
        if any(token in rel_lower for token in ("store", "zustand", "redux", "state")):
            state_files.append(path)
        if any(token in rel_lower for token in ("form", "schema", "zod")):
            form_files.append(path)
        if not should_consider_text_file(path):
            continue
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue
        for match in _COMPONENT_RE.finditer(text):
            name = next((group for group in match.groups() if group), "")
            if name:
                components.append((path, name))
        for match in _HOOK_RE.finditer(text):
            name = next((group for group in match.groups() if group), "")
            if name:
                hooks.append((path, name))

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# Frontend Report\n\nGenerated: {human_now()}\n\n")
        out.write("## Frontend libraries detected from package.json\n\n")
        interesting = ["react", "vue", "svelte", "@tanstack/react-router", "@tanstack/react-query", "react-hook-form", "zod", "zustand", "redux", "tailwindcss", "framer-motion", "recharts", "echarts"]
        detected = [name for name in interesting if name in deps]
        if detected:
            for name in detected:
                out.write(f"- `{name}` — `{deps[name]}`\n")
        else:
            out.write("- No common frontend libraries detected.\n")

        out.write("\n## Important frontend directories\n\n")
        for role, dirs in dirs_by_role.items():
            out.write(f"### {role}\n")
            if dirs:
                for directory in sorted(set(dirs), key=lambda p: str(p).lower())[:60]:
                    out.write(f"- `{rel_display(directory, copied_root)}`\n")
            else:
                out.write("- not detected\n")
            out.write("\n")

        groups = (("Route/page files", route_files), ("State/store files", state_files), ("Form/schema files", form_files))
        for title, paths in groups:
            out.write(f"## {title}\n\n")
            if paths:
                for path in sorted(set(paths), key=lambda p: str(p).lower())[:120]:
                    out.write(f"- `{rel_display(path, copied_root)}`\n")
            else:
                out.write("- none detected\n")
            out.write("\n")

        out.write("## Component candidates\n\n")
        if components:
            for path, name in sorted(set(components), key=lambda item: (item[1].lower(), str(item[0]).lower()))[:200]:
                out.write(f"- `{name}` — `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- none detected\n")

        out.write("\n## Hook candidates\n\n")
        if hooks:
            for path, name in sorted(set(hooks), key=lambda item: (item[1].lower(), str(item[0]).lower()))[:200]:
                out.write(f"- `{name}` — `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- none detected\n")


def write_backend_report(copied_root: Path, output_file: Path, max_bytes_per_file: int) -> None:
    backend_dirs: dict[str, list[Path]] = {"api": [], "services": [], "models": [], "repositories": [], "migrations": [], "workers": [], "config": []}
    for directory in iter_project_dirs(copied_root):
        parts = {part.lower() for part in directory.relative_to(copied_root).parts}
        for key in backend_dirs:
            if key in parts or (key == "workers" and {"tasks", "jobs", "worker"}.intersection(parts)):
                backend_dirs[key].append(directory)

    py_symbols: list[tuple[Path, str, str]] = []
    go_files: list[Path] = []
    db_files: list[Path] = []
    config_files: list[Path] = []

    for path in iter_project_files(copied_root):
        ext = extension_key(path)
        rel_lower = str(path.relative_to(copied_root)).lower().replace("\\", "/")
        if ext == "go":
            go_files.append(path)
        if ext in {"sql", "prisma"} or "migration" in rel_lower:
            db_files.append(path)
        if any(token in path.name.lower() for token in ("config", "settings")):
            config_files.append(path)
        if ext == "py" and should_consider_text_file(path):
            text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
            if text is None:
                continue
            for name in _PY_CLASS_RE.findall(text):
                py_symbols.append((path, "class", name))
            for name in _PY_FUNC_RE.findall(text):
                py_symbols.append((path, "def", name))

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# Backend Report\n\nGenerated: {human_now()}\n\n")
        out.write("## Backend directories\n\n")
        for role, dirs in backend_dirs.items():
            out.write(f"### {role}\n")
            if dirs:
                for directory in sorted(set(dirs), key=lambda p: str(p).lower())[:80]:
                    out.write(f"- `{rel_display(directory, copied_root)}`\n")
            else:
                out.write("- not detected\n")
            out.write("\n")

        groups = (("Go files", go_files), ("Database/migration files", db_files), ("Config/settings files", config_files))
        for title, paths in groups:
            out.write(f"## {title}\n\n")
            if paths:
                for path in sorted(set(paths), key=lambda p: str(p).lower())[:120]:
                    out.write(f"- `{rel_display(path, copied_root)}`\n")
            else:
                out.write("- none detected\n")
            out.write("\n")

        out.write("## Python class/function candidates\n\n")
        if py_symbols:
            for path, kind, name in sorted(set(py_symbols), key=lambda item: (str(item[0]).lower(), item[2].lower()))[:250]:
                out.write(f"- `{kind} {name}` — `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- none detected\n")
