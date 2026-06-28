from __future__ import annotations

from pathlib import Path

from ...constants import CONFIG_FILES
from ...utils.inventory import iter_project_files
from ...utils.path_utils import rel_display
from ...utils.text_utils import format_bytes
from ...utils.time_utils import human_now


def find_config_files(copied_root: Path) -> list[Path]:
    known: list[Path] = []
    for path in iter_project_files(copied_root):
        rel = str(path.relative_to(copied_root)).replace("\\", "/")
        name = path.name
        lower_name = name.lower()
        if name in CONFIG_FILES or lower_name in {item.lower() for item in CONFIG_FILES}:
            known.append(path)
            continue
        if rel.startswith(".github/workflows/") and path.suffix.lower() in {
            ".yml",
            ".yaml",
        }:
            known.append(path)
            continue
        if lower_name.startswith("dockerfile"):
            known.append(path)
            continue
        if lower_name.startswith(".env"):
            known.append(path)
            continue
    return sorted(set(known), key=lambda p: str(p).lower())


def write_config_report(copied_root: Path, output_file: Path) -> None:
    configs = find_config_files(copied_root)
    names = {p.name.lower() for p in configs}

    capabilities = {
        "TypeScript": "tsconfig.json" in names,
        "Vite": any(p.name.lower().startswith("vite.config") for p in configs),
        "ESLint": any(
            "eslint" in p.name.lower() or p.name.lower().startswith(".eslintrc") for p in configs
        ),
        "Prettier": any(
            "prettier" in p.name.lower() or p.name.lower().startswith(".prettierrc")
            for p in configs
        ),
        "Tailwind CSS": any(p.name.lower().startswith("tailwind.config") for p in configs),
        "PostCSS": any(p.name.lower().startswith("postcss.config") for p in configs),
        "Docker": any(p.name.lower().startswith("dockerfile") for p in configs),
        "Docker Compose": any(
            p.name.lower()
            in {
                "docker-compose.yml",
                "docker-compose.yaml",
                "compose.yml",
                "compose.yaml",
            }
            for p in configs
        ),
        "GitHub Actions": any(
            str(p.relative_to(copied_root)).replace("\\", "/").startswith(".github/workflows/")
            for p in configs
        ),
        "Python pyproject": "pyproject.toml" in names,
        "Go modules": "go.mod" in names,
        "Rust Cargo": "cargo.toml" in names,
        "Environment examples": any(p.name.lower().startswith(".env") for p in configs),
    }

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Configuration Report ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("=" * 100 + "\n\n")

        out.write("--- Capability checklist ---\n")
        for name, present in capabilities.items():
            out.write(f"{name:<24} {'yes' if present else 'no'}\n")

        out.write("\n--- Detected configuration files ---\n")
        if configs:
            for path in configs:
                try:
                    size = format_bytes(path.stat().st_size)
                except Exception:
                    size = "unknown"
                out.write(f"{size:>12}  {rel_display(path, copied_root)}\n")
        else:
            out.write("None detected.\n")