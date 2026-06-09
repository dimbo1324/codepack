from __future__ import annotations

import re
from pathlib import Path

from ...utils.text_utils import safe_read_json
from ...utils.time_utils import human_now


def write_scripts_report(copied_root: Path, output_file: Path) -> None:
    package_json = safe_read_json(copied_root / "package.json")

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Scripts and Common Commands Report ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("=" * 100 + "\n\n")

        scripts = package_json.get("scripts") if package_json else None
        out.write("--- package.json scripts ---\n")
        if isinstance(scripts, dict) and scripts:
            manager = "pnpm" if (copied_root / "pnpm-lock.yaml").exists() else "npm"
            for name, command in sorted(scripts.items(), key=lambda item: item[0].lower()):
                out.write(f"{manager} run {name:<24} # {command}\n")
        else:
            out.write("No package.json scripts detected.\n")

        out.write("\n--- Makefile targets ---\n")
        makefile = next(
            (p for p in (copied_root / "Makefile", copied_root / "makefile") if p.exists()),
            None,
        )
        if makefile:
            try:
                targets = []
                for line in makefile.read_text(encoding="utf-8", errors="replace").splitlines():
                    match = re.match(r"^([A-Za-z0-9_.-]+):(?:\s|$)", line)
                    if match and not line.startswith("\t"):
                        targets.append(match.group(1))
                for target in sorted(set(targets))[:200]:
                    out.write(f"make {target}\n")
            except Exception as exc:
                out.write(f"Could not read Makefile: {exc}\n")
        else:
            out.write("No Makefile detected.\n")

        out.write("\n--- Docker convenience commands ---\n")
        if (copied_root / "docker-compose.yml").exists() or (
            copied_root / "docker-compose.yaml"
        ).exists():
            out.write("docker compose up --build\n")
            out.write("docker compose down\n")
            out.write("docker compose logs -f\n")
        elif list(copied_root.glob("Dockerfile*")):
            out.write(
                "Dockerfile detected. Add a project-specific docker build command if needed.\n"
            )
        else:
            out.write("No Docker/Docker Compose files detected.\n")
