"""Dependency report: extracts package lists from package.json, requirements*.txt, go.mod, and Cargo.toml.

Writes 03_dependencies.txt with a plain-text summary suitable for quick scanning.
parse_go_mod() is also reused by dependency_intelligence.py.
"""

from __future__ import annotations

from pathlib import Path

from ...utils.inventory import detect_package_managers
from ...utils.text_utils import safe_read_json
from ...utils.time_utils import human_now


def parse_go_mod(path: Path) -> tuple[str, list[str]]:
    module_name = ""
    requirements: list[str] = []
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except Exception:
        return module_name, requirements

    # Hand-rolled parser avoids importing a TOML/Go-module library; handles both
    # single-line `require pkg v1.0` and block `require ( ... )` syntax.
    in_require_block = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("module "):
            module_name = stripped.split(maxsplit=1)[1]
        elif stripped == "require (":
            in_require_block = True
        elif in_require_block and stripped == ")":
            in_require_block = False
        elif stripped.startswith("require "):
            requirements.append(stripped.removeprefix("require ").strip())
        elif in_require_block and stripped and not stripped.startswith("//"):
            requirements.append(stripped)
    return module_name, requirements


def write_dependency_report(copied_root: Path, output_file: Path) -> None:
    package_json_path = copied_root / "package.json"
    package_json = safe_read_json(package_json_path)

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Dependency Report ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("=" * 100 + "\n\n")

        managers = detect_package_managers(copied_root)
        out.write(
            f"Detected package managers: {', '.join(managers) if managers else 'not detected'}\n\n"
        )

        if package_json:
            out.write("--- package.json metadata ---\n")
            for key in ("name", "version", "type", "private", "packageManager"):
                if key in package_json:
                    out.write(f"{key:<20}: {package_json[key]}\n")
            engines = package_json.get("engines")
            if isinstance(engines, dict):
                out.write("engines:\n")
                for key, value in sorted(engines.items()):
                    out.write(f"  - {key}: {value}\n")

            for section in (
                "dependencies",
                "devDependencies",
                "peerDependencies",
                "optionalDependencies",
            ):
                deps = package_json.get(section)
                out.write(f"\n--- {section} ---\n")
                if isinstance(deps, dict) and deps:
                    for name, version in sorted(deps.items(), key=lambda item: item[0].lower()):
                        out.write(f"{name:<45} {version}\n")
                else:
                    out.write("None.\n")
        else:
            out.write("package.json: not found or unreadable.\n")

        requirements_files = sorted(copied_root.glob("requirements*.txt"))
        if requirements_files:
            out.write("\n--- Python requirements files ---\n")
            for req_file in requirements_files:
                out.write(f"\nFile: {req_file.name}\n")
                try:
                    lines = [
                        line.strip()
                        for line in req_file.read_text(
                            encoding="utf-8", errors="replace"
                        ).splitlines()
                        if line.strip() and not line.strip().startswith("#")
                    ]
                except Exception as exc:
                    out.write(f"Could not read: {exc}\n")
                    continue
                for line in lines[:300]:
                    out.write(f"- {line}\n")
                if len(lines) > 300:
                    out.write(f"... and {len(lines) - 300:,} more\n")

        if (copied_root / "go.mod").exists():
            out.write("\n--- Go modules ---\n")
            module_name, requirements = parse_go_mod(copied_root / "go.mod")
            out.write(f"module: {module_name or 'not detected'}\n")
            for requirement in requirements[:300]:
                out.write(f"- {requirement}\n")
            if len(requirements) > 300:
                out.write(f"... and {len(requirements) - 300:,} more\n")

        if (copied_root / "Cargo.toml").exists():
            out.write("\n--- Rust Cargo ---\n")
            # Full Cargo.toml parsing is skipped on purpose to keep the codebase stdlib-only.
            out.write(
                "Cargo.toml detected. Full TOML parsing is intentionally not performed without external dependencies.\n"
            )