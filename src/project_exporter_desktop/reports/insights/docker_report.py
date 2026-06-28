"""Docker / infrastructure report: lists Dockerfiles and parses docker-compose service definitions.

Uses a hand-written indent-sensitive parser so no PyYAML dependency is required.
Writes 10_docker.txt.  Environment values are redacted before being written.
"""

from __future__ import annotations

from collections import defaultdict
from pathlib import Path

from ...utils.inventory import iter_project_files
from ...utils.path_utils import rel_display
from ...utils.time_utils import human_now
from .security import redacted_line


def parse_compose_services(text: str) -> dict[str, dict[str, list[str]]]:
    services: dict[str, dict[str, list[str]]] = {}
    lines = text.splitlines()
    in_services = False
    current_service = ""
    current_key = ""

    for raw_line in lines:
        if not raw_line.strip() or raw_line.lstrip().startswith("#"):
            continue
        indent = len(raw_line) - len(raw_line.lstrip(" "))
        stripped = raw_line.strip()
        if indent == 0 and stripped.startswith("services:"):
            in_services = True
            current_service = ""
            continue
        # Any top-level key after "services:" signals the end of the services block.
        if indent == 0 and in_services and not stripped.startswith("services:"):
            break
        if not in_services:
            continue
        if indent == 2 and stripped.endswith(":"):
            current_service = stripped[:-1].strip().strip("\"'")
            services.setdefault(current_service, defaultdict(list))
            current_key = ""
            continue
        if current_service and indent == 4 and ":" in stripped:
            key, value = stripped.split(":", 1)
            current_key = key.strip()
            value = value.strip()
            if value:
                services[current_service].setdefault(current_key, []).append(value)
            continue
        if current_service and indent >= 6 and stripped.startswith("-") and current_key:
            services[current_service].setdefault(current_key, []).append(stripped[1:].strip())
    return services


def write_docker_report(copied_root: Path, output_file: Path) -> None:
    dockerfiles = sorted(
        [p for p in iter_project_files(copied_root) if p.name.lower().startswith("dockerfile")],
        key=lambda p: str(p).lower(),
    )
    compose_files = sorted(
        [
            p
            for p in iter_project_files(copied_root)
            if p.name.lower()
            in {
                "docker-compose.yml",
                "docker-compose.yaml",
                "compose.yml",
                "compose.yaml",
            }
        ],
        key=lambda p: str(p).lower(),
    )

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Docker / Infrastructure Report ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("Compose parsing is heuristic and works best with simple YAML files.\n")
        out.write("=" * 100 + "\n\n")

        out.write("--- Dockerfiles ---\n")
        if dockerfiles:
            for path in dockerfiles:
                out.write(f"{rel_display(path, copied_root)}\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Compose files ---\n")
        if compose_files:
            for path in compose_files:
                out.write(f"{rel_display(path, copied_root)}\n")
        else:
            out.write("None detected.\n")

        for compose in compose_files:
            out.write(f"\n--- Parsed services from {rel_display(compose, copied_root)} ---\n")
            try:
                text = compose.read_text(encoding="utf-8", errors="replace")
            except Exception as exc:
                out.write(f"Could not read compose file: {exc}\n")
                continue
            services = parse_compose_services(text)
            if not services:
                out.write("No services parsed.\n")
                continue
            for service, data in services.items():
                out.write(f"\nService: {service}\n")
                for key in (
                    "image",
                    "build",
                    "ports",
                    "volumes",
                    "environment",
                    "env_file",
                    "depends_on",
                ):
                    values = data.get(key, [])
                    if values:
                        out.write(f"  {key}:\n")
                        for value in values[:80]:
                            safe_value = (
                                redacted_line(value)
                                if key in {"environment", "env_file"}
                                else value
                            )
                            out.write(f"    - {safe_value}\n")