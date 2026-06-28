from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from ...utils.inventory import detect_package_managers
from ...utils.path_utils import rel_display
from ...utils.text_utils import safe_read_json
from ...utils.time_utils import human_now
from .config_report import find_config_files


def _flatten_stack(stack: dict[str, list[str]]) -> list[str]:
    values: list[str] = []
    for group_values in stack.values():
        for value in group_values:
            if value not in values:
                values.append(value)
    return sorted(values, key=str.lower)


def _detect_project_type(root: Path, stack: dict[str, list[str]], files: list[Path]) -> str:
    has_frontend = bool(stack.get("frontend")) or any(
        (root / name).exists()
        for name in ("package.json", "vite.config.ts", "vite.config.js", "next.config.js")
    )
    has_backend = bool(stack.get("backend")) or any(
        (root / name).exists()
        for name in ("pyproject.toml", "requirements.txt", "go.mod", "Cargo.toml")
    )
    has_tkinter = False
    for path in files:
        if path.suffix.lower() not in {".py", ".pyw"}:
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="replace")[:20000]
        except Exception:
            continue
        if "import tkinter" in text or "from tkinter" in text:
            has_tkinter = True
            break

    if has_frontend and has_backend:
        return "fullstack"
    if has_tkinter:
        return "python_desktop_tool"
    if has_frontend:
        return "frontend"
    if has_backend:
        return "backend_or_cli"
    return "unknown"


def _detect_entrypoints(root: Path) -> list[str]:
    candidates = [
        "main.py",
        "app.py",
        "server.py",
        "manage.py",
        "src/main.py",
        "src/app.py",
        "src/__main__.py",
        "src/index.ts",
        "src/index.tsx",
        "src/main.ts",
        "src/main.tsx",
        "src/App.tsx",
        "cmd/main.go",
        "main.go",
    ]
    found: list[str] = []
    for rel in candidates:
        path = root / rel
        if path.exists() and path.is_file():
            found.append(rel.replace("/", "\\"))

    package_json = safe_read_json(root / "package.json")
    for key in ("main", "module", "bin"):
        value = package_json.get(key)
        if isinstance(value, str):
            found.append(f"package.json:{key} -> {value}")
        elif isinstance(value, dict):
            for name, target in sorted(value.items()):
                found.append(f"package.json:{key}.{name} -> {target}")
    return sorted(set(found), key=str.lower)


def _detect_commands(root: Path) -> dict[str, list[str]]:
    commands: dict[str, list[str]] = {"install": [], "dev": [], "build": [], "test": [], "run": []}
    managers = detect_package_managers(root)
    package_json = safe_read_json(root / "package.json")
    scripts = package_json.get("scripts")
    node_manager = (
        "pnpm"
        if "pnpm" in managers
        else "npm"
        if "npm" in managers
        else "yarn"
        if "Yarn" in managers
        else "npm"
    )
    if package_json:
        if node_manager == "pnpm":
            commands["install"].append("pnpm install")
        elif node_manager == "yarn":
            commands["install"].append("yarn install")
        else:
            commands["install"].append("npm install")
    if isinstance(scripts, dict):
        for name in sorted(scripts):
            command = f"{node_manager} run {name}"
            lname = name.lower()
            if "dev" in lname or "start" in lname:
                commands["dev"].append(command)
            elif "build" in lname:
                commands["build"].append(command)
            elif "test" in lname or "check" in lname or "lint" in lname:
                commands["test"].append(command)
            else:
                commands["run"].append(command)

    if (root / "requirements.txt").exists():
        commands["install"].append("python -m venv .venv")
        commands["install"].append(".venv\\Scripts\\python -m pip install -r requirements.txt")
    if (root / "pyproject.toml").exists():
        commands["install"].append("python -m pip install -e .")
        commands["test"].append("python -m pytest")
    if (root / "go.mod").exists():
        commands["install"].append("go mod download")
        commands["build"].append("go build ./...")
        commands["test"].append("go test ./...")
    if (root / "Cargo.toml").exists():
        commands["build"].append("cargo build")
        commands["test"].append("cargo test")
    if (
        (root / "docker-compose.yml").exists()
        or (root / "docker-compose.yaml").exists()
        or (root / "compose.yml").exists()
        or (root / "compose.yaml").exists()
    ):
        commands["run"].append("docker compose up --build")
    elif any(path.name.lower().startswith("dockerfile") for path in root.glob("Dockerfile*")):
        commands["build"].append("docker build -t <image-name> .")
    return {key: value for key, value in commands.items() if value}


def _detect_risk_level(root: Path, files: list[Path]) -> tuple[str, list[str]]:
    reasons: list[str] = []
    env_files = [p for p in files if p.name.lower().startswith(".env")]
    if env_files:
        reasons.append(f"{len(env_files)} .env-like file(s) detected")
    if not any(
        re.search(r"(?i)(^|[._/-])(test|spec)([._/-]|$)", str(p.relative_to(root))) for p in files
    ):
        reasons.append("no obvious test files detected")
    if not ((root / ".github" / "workflows").exists()):
        reasons.append("no GitHub Actions workflow detected")
    if any(p.name.lower() in {"id_rsa", "id_ed25519", "credentials.json"} for p in files):
        reasons.append("sensitive credential-like filename detected")
    if len(reasons) >= 3:
        return "high", reasons
    if reasons:
        return "medium", reasons
    return "low", ["no obvious high-level project hygiene risks detected"]


def build_project_profile(
    copied_root: Path, source_root: Path, inventory: dict[str, Any]
) -> dict[str, Any]:
    files: list[Path] = inventory["files"]
    stack: dict[str, list[str]] = inventory["stack"]
    configs = find_config_files(copied_root)
    risk_level, risk_reasons = _detect_risk_level(copied_root, files)
    docker_compose_files = [
        p.name
        for p in configs
        if p.name.lower()
        in {"docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"}
    ]
    package_json = safe_read_json(copied_root / "package.json")
    scripts = package_json.get("scripts") if package_json else None

    return {
        "schema_version": 1,
        "generated_at": human_now(),
        "project_name": copied_root.name,
        "source_root": str(source_root),
        "copied_root": str(copied_root),
        "project_type": _detect_project_type(copied_root, stack, files),
        "detected_stack": _flatten_stack(stack),
        "stack_by_group": stack,
        "package_managers": detect_package_managers(copied_root),
        "entrypoints": _detect_entrypoints(copied_root),
        "commands": _detect_commands(copied_root),
        "docker_compose_files": docker_compose_files,
        "npm_scripts": scripts if isinstance(scripts, dict) else {},
        "capabilities": {
            "has_git": (source_root / ".git").exists(),
            "has_ci": (copied_root / ".github" / "workflows").exists(),
            "has_tests": any(
                re.search(r"(?i)(^|[._/-])(test|spec)([._/-]|$)", str(p.relative_to(copied_root)))
                for p in files
            ),
            "has_docker": bool(docker_compose_files)
            or any(p.name.lower().startswith("dockerfile") for p in configs),
            "has_env_files": any(p.name.lower().startswith(".env") for p in files),
            "has_readme": any(p.name.lower().startswith("readme") for p in files),
            "has_license": any(p.name.lower().startswith("license") for p in files),
        },
        "counts": {
            "files": len(files),
            "folders": len(inventory["dirs"]),
            "total_size_bytes": int(inventory["total_size"]),
        },
        "risk_level": risk_level,
        "risk_reasons": risk_reasons,
        "important_config_files": [rel_display(p, copied_root) for p in configs[:100]],
    }


def write_project_profile_json(
    copied_root: Path, source_root: Path, output_file: Path, inventory: dict[str, Any]
) -> None:
    profile = build_project_profile(copied_root, source_root, inventory)
    output_file.write_text(json.dumps(profile, ensure_ascii=False, indent=2), encoding="utf-8")