# Insight report generator: reads the copied project and writes one focused analysis artifact into reports/insights.

from __future__ import annotations

from pathlib import Path

from ...utils.inventory import detect_package_managers
from ...utils.path_utils import rel_display
from ...utils.text_utils import safe_read_json
from ...utils.time_utils import human_now


def _detect_node_manager(root: Path) -> str:
    if (root / "pnpm-lock.yaml").exists():
        return "pnpm"
    if (root / "yarn.lock").exists():
        return "yarn"
    if (root / "bun.lockb").exists() or (root / "bun.lock").exists():
        return "bun"
    return "npm"


def _compose_files(root: Path) -> list[Path]:
    return [
        path
        for path in (
            root / name
            for name in ("docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml")
        )
        if path.exists()
    ]


def write_runbook_report(copied_root: Path, output_file: Path) -> None:
    package_json = safe_read_json(copied_root / "package.json")
    scripts = package_json.get("scripts") if package_json else None
    managers = detect_package_managers(copied_root)
    node_manager = _detect_node_manager(copied_root)
    compose_files = _compose_files(copied_root)
    env_examples = sorted(copied_root.glob(".env*"), key=lambda p: p.name.lower())

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# Runbook: {copied_root.name}\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write(
            "This runbook is generated heuristically from common project files. Verify commands before using them in production.\n\n"
        )

        out.write("## Detected package / build systems\n\n")
        if managers:
            for manager in managers:
                out.write(f"- {manager}\n")
        else:
            out.write("- No common package manager detected.\n")

        out.write("\n## Setup commands\n\n")
        commands: list[str] = []
        if package_json:
            if node_manager == "pnpm":
                commands.append("pnpm install")
            elif node_manager == "yarn":
                commands.append("yarn install")
            elif node_manager == "bun":
                commands.append("bun install")
            else:
                commands.append("npm install")
        if (copied_root / "requirements.txt").exists():
            commands.extend(
                [
                    "python -m venv .venv",
                    ".venv\\Scripts\\python -m pip install -r requirements.txt",
                ]
            )
        if (copied_root / "pyproject.toml").exists():
            commands.append("python -m pip install -e .")
        if (copied_root / "go.mod").exists():
            commands.append("go mod download")
        if (copied_root / "Cargo.toml").exists():
            commands.append("cargo fetch")
        if commands:
            for command in commands:
                out.write(f"```powershell\n{command}\n```\n\n")
        else:
            out.write("No setup commands detected.\n\n")

        out.write("## Development / run commands\n\n")
        if isinstance(scripts, dict) and scripts:
            for name, command in sorted(scripts.items(), key=lambda item: item[0].lower()):
                out.write(f"- `{node_manager} run {name}` → `{command}`\n")
        elif (copied_root / "main.py").exists():
            out.write("```powershell\npython main.py\n```\n")
        else:
            out.write("No obvious development command detected.\n")

        out.write("\n## Test / check commands\n\n")
        test_commands: list[str] = []
        if isinstance(scripts, dict):
            for name in scripts:
                if any(token in name.lower() for token in ("test", "check", "lint", "type")):
                    test_commands.append(f"{node_manager} run {name}")
        if (copied_root / "pyproject.toml").exists() or (copied_root / "pytest.ini").exists():
            test_commands.append("python -m pytest")
        if (copied_root / "go.mod").exists():
            test_commands.append("go test ./...")
        if (copied_root / "Cargo.toml").exists():
            test_commands.append("cargo test")
        if test_commands:
            for command in sorted(set(test_commands)):
                out.write(f"- `{command}`\n")
        else:
            out.write("- No obvious test/check command detected.\n")

        out.write("\n## Docker commands\n\n")
        if compose_files:
            for path in compose_files:
                out.write(f"- Compose file: `{rel_display(path, copied_root)}`\n")
            out.write("\n```powershell\ndocker compose up --build\n```\n")
        elif any(
            path.name.lower().startswith("dockerfile") for path in copied_root.glob("Dockerfile*")
        ):
            out.write("```powershell\ndocker build -t <image-name> .\n```\n")
        else:
            out.write("No Dockerfile/docker-compose file detected.\n")

        out.write("\n## Environment/configuration hints\n\n")
        if env_examples:
            for path in env_examples:
                out.write(f"- `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- No .env-like files detected.\n")
