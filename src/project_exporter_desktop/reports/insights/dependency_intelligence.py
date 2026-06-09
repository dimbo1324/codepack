from __future__ import annotations

from pathlib import Path

from ...utils.inventory import detect_package_managers
from ...utils.text_utils import safe_read_json
from ...utils.time_utils import human_now
from .dependencies import parse_go_mod


def _section(out, title: str) -> None:
    out.write(f"\n## {title}\n\n")


def write_dependency_intelligence_report(copied_root: Path, output_file: Path) -> None:
    package_json = safe_read_json(copied_root / "package.json")
    managers = detect_package_managers(copied_root)

    with output_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write("# Dependency Intelligence\n")
        out.write(f"\nGenerated: {human_now()}\n")

        _section(out, "Detected package managers")
        if managers:
            for manager in managers:
                out.write(f"- {manager}\n")
        else:
            out.write("No known package manager files detected.\n")

        _section(out, "JavaScript / TypeScript")
        if package_json:
            out.write("package.json detected.\n\n")
            for section in (
                "dependencies",
                "devDependencies",
                "peerDependencies",
                "optionalDependencies",
            ):
                deps = package_json.get(section)
                out.write(f"### {section}\n\n")
                if isinstance(deps, dict) and deps:
                    for name, version in sorted(deps.items(), key=lambda item: item[0].casefold()):
                        out.write(f"- `{name}`: `{version}`\n")
                else:
                    out.write("None.\n")
                out.write("\n")
            lockfiles = [
                name
                for name in (
                    "pnpm-lock.yaml",
                    "package-lock.json",
                    "yarn.lock",
                    "bun.lock",
                    "bun.lockb",
                )
                if (copied_root / name).exists()
            ]
            out.write(
                "Lockfile status: "
                + (
                    ", ".join(lockfiles)
                    if lockfiles
                    else "missing — add one for reproducible installs"
                )
                + "\n"
            )
        else:
            out.write("package.json not detected.\n")

        _section(out, "Python")
        py_files = [
            name
            for name in (
                "pyproject.toml",
                "requirements.txt",
                "requirements-dev.txt",
                "poetry.lock",
                "Pipfile",
                "Pipfile.lock",
                "uv.lock",
            )
            if (copied_root / name).exists()
        ]
        if py_files:
            for name in py_files:
                out.write(f"- `{name}`\n")
        else:
            out.write("No Python dependency metadata detected.\n")

        _section(out, "Go")
        if (copied_root / "go.mod").exists():
            module, requirements = parse_go_mod(copied_root / "go.mod")
            out.write(f"module: `{module or 'not detected'}`\n\n")
            for requirement in requirements[:100]:
                out.write(f"- `{requirement}`\n")
            if len(requirements) > 100:
                out.write(f"- ... and {len(requirements) - 100:,} more\n")
        else:
            out.write("go.mod not detected.\n")

        _section(out, "Hygiene checks")
        if package_json and not any(
            (copied_root / name).exists()
            for name in (
                "pnpm-lock.yaml",
                "package-lock.json",
                "yarn.lock",
                "bun.lock",
                "bun.lockb",
            )
        ):
            out.write("- Add a JS lockfile for reproducibility.\n")
        if (copied_root / "requirements.txt").exists() and not (
            copied_root / "pyproject.toml"
        ).exists():
            out.write(
                "- Consider adding `pyproject.toml` for tool configuration and packaging metadata.\n"
            )
        if not managers:
            out.write(
                "- No dependency action required unless this project is expected to be installable.\n"
            )
