from __future__ import annotations

import re
from pathlib import Path

from ...utils.inventory import extension_key, iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import read_text_safely, should_consider_text_file
from ...utils.time_utils import human_now

_FASTAPI_RE = re.compile(r"@(?:app|router)\.(get|post|put|patch|delete|options|head)\(\s*['\"]([^'\"]+)", re.IGNORECASE)
_FLASK_RE = re.compile(r"@(?:app|blueprint|bp)\.route\(\s*['\"]([^'\"]+).*?(?:methods\s*=\s*\[([^\]]+)\])?", re.IGNORECASE)
_EXPRESS_RE = re.compile(r"(?:app|router)\.(get|post|put|patch|delete|use)\(\s*['\"]([^'\"]+)", re.IGNORECASE)
_GO_RE = re.compile(r"(?:http\.)?HandleFunc\(\s*['\"]([^'\"]+)")
_FETCH_RE = re.compile(r"(?:fetch|axios\.(?:get|post|put|patch|delete)|client\.(?:get|post|put|patch|delete))\(\s*`?['\"]?([^'\"`)]+)")
_OPENAPI_NAMES = {"openapi.yaml", "openapi.yml", "openapi.json", "swagger.yaml", "swagger.yml", "swagger.json"}


def write_api_surface_report(copied_root: Path, output_file: Path, max_bytes_per_file: int | None) -> None:
    backend_routes: list[tuple[Path, str, str]] = []
    frontend_calls: list[tuple[Path, str]] = []
    specs: list[Path] = []

    for path in iter_project_files(copied_root):
        name = path.name.lower()
        if name in _OPENAPI_NAMES:
            specs.append(path)
        ext = extension_key(path)
        if ext not in {"py", "js", "jsx", "ts", "tsx", "go", "vue", "svelte", "astro"}:
            continue
        if not should_consider_text_file(path):
            continue
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue

        if ext == "py":
            for method, route in _FASTAPI_RE.findall(text):
                backend_routes.append((path, method.upper(), route))
            for route, methods in _FLASK_RE.findall(text):
                method_text = methods or "GET(default)"
                backend_routes.append((path, method_text.replace("'", "").replace('"', ""), route))
        elif ext in {"js", "jsx", "ts", "tsx", "vue", "svelte", "astro"}:
            for method, route in _EXPRESS_RE.findall(text):
                backend_routes.append((path, method.upper(), route))
            for call in _FETCH_RE.findall(text):
                if call.startswith(("http", "/", "api", "${")) or "/api" in call:
                    frontend_calls.append((path, call))
        elif ext == "go":
            for route in _GO_RE.findall(text):
                backend_routes.append((path, "GO_HANDLEFUNC", route))

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# API Surface Report\n\nGenerated: {human_now()}\n\n")
        out.write("This report detects backend routes and frontend HTTP calls using conservative regex heuristics.\n\n")

        out.write("## API specifications\n\n")
        if specs:
            for path in sorted(specs, key=lambda p: str(p).lower()):
                out.write(f"- `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- No OpenAPI/Swagger files detected.\n")

        out.write("\n## Backend route candidates\n\n")
        if backend_routes:
            for path, method, route in sorted(set(backend_routes), key=lambda item: (item[2], str(item[0]).lower()))[:500]:
                out.write(f"- `{method}` `{route}` — `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- No backend route candidates detected.\n")

        out.write("\n## Frontend HTTP call candidates\n\n")
        if frontend_calls:
            for path, call in sorted(set(frontend_calls), key=lambda item: (item[1], str(item[0]).lower()))[:500]:
                out.write(f"- `{call}` — `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- No frontend HTTP call candidates detected.\n")
