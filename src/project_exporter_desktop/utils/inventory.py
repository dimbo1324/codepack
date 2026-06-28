# Small utility module shared across services, reports, GUI workers, and tests.

from __future__ import annotations

import os
from collections import Counter
from collections.abc import Iterable
from pathlib import Path
from typing import Any

from ..constants import LANGUAGE_BY_EXTENSION
from .path_utils import should_ignore_dir
from .text_utils import safe_read_json


def iter_project_files(root: Path) -> Iterable[Path]:
    for current_dir, dirnames, filenames in os.walk(root, topdown=True, followlinks=False):
        dirnames[:] = [
            dirname
            for dirname in dirnames
            if not should_ignore_dir(dirname) and not (Path(current_dir) / dirname).is_symlink()
        ]
        for filename in filenames:
            path = Path(current_dir) / filename
            if path.is_symlink():
                continue
            yield path


def iter_project_dirs(root: Path) -> Iterable[Path]:
    for current_dir, dirnames, _filenames in os.walk(root, topdown=True, followlinks=False):
        current = Path(current_dir)
        dirnames[:] = [
            dirname
            for dirname in dirnames
            if not should_ignore_dir(dirname) and not (current / dirname).is_symlink()
        ]
        for dirname in dirnames:
            yield current / dirname


def extension_key(path: Path) -> str:
    name = path.name.lower()
    if name == "dockerfile" or name.startswith("dockerfile."):
        return "dockerfile"
    suffix = path.suffix.lower().lstrip(".")
    return suffix or "[no extension]"


def is_non_ascii(text: str) -> bool:
    return any(ord(ch) > 127 for ch in text)


def path_depth(path: Path, root: Path) -> int:
    try:
        return len(path.relative_to(root).parts)
    except Exception:
        return 0


def detect_package_managers(root: Path) -> list[str]:
    managers: list[str] = []
    if (root / "pnpm-lock.yaml").exists():
        managers.append("pnpm")
    if (root / "package-lock.json").exists():
        managers.append("npm")
    if (root / "yarn.lock").exists():
        managers.append("Yarn")
    if (root / "bun.lockb").exists() or (root / "bun.lock").exists():
        managers.append("Bun")
    if (root / "poetry.lock").exists():
        managers.append("Poetry")
    if (root / "Pipfile").exists():
        managers.append("Pipenv")
    if (root / "requirements.txt").exists():
        managers.append("pip/requirements.txt")
    if (root / "go.mod").exists():
        managers.append("Go modules")
    if (root / "Cargo.toml").exists():
        managers.append("Cargo")
    return managers


def package_json_dependencies(root: Path) -> dict[str, str]:
    package_json = safe_read_json(root / "package.json")
    deps: dict[str, str] = {}
    for section in (
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ):
        value = package_json.get(section)
        if isinstance(value, dict):
            deps.update({str(k): str(v) for k, v in value.items()})
    return deps


def detect_stack(root: Path) -> dict[str, list[str]]:
    deps = package_json_dependencies(root)
    dep_names = set(deps)
    files = {p.name.lower() for p in root.iterdir() if p.exists()} if root.exists() else set()

    frontend: list[str] = []
    backend: list[str] = []
    tools: list[str] = []
    testing: list[str] = []
    styling: list[str] = []
    infrastructure: list[str] = []

    checks = [
        ("react", "React", frontend),
        ("@vitejs/plugin-react", "Vite React plugin", frontend),
        ("vite", "Vite", tools),
        ("next", "Next.js", frontend),
        ("vue", "Vue", frontend),
        ("nuxt", "Nuxt", frontend),
        ("svelte", "Svelte", frontend),
        ("@sveltejs/kit", "SvelteKit", frontend),
        ("astro", "Astro", frontend),
        ("typescript", "TypeScript", tools),
        ("tailwindcss", "Tailwind CSS", styling),
        ("@tailwindcss/vite", "Tailwind CSS Vite plugin", styling),
        ("sass", "Sass", styling),
        ("less", "Less", styling),
        ("zustand", "Zustand", frontend),
        ("zod", "Zod", frontend),
        ("@tanstack/react-query", "TanStack Query", frontend),
        ("@tanstack/react-router", "TanStack Router", frontend),
        ("react-router", "React Router", frontend),
        ("react-hook-form", "React Hook Form", frontend),
        ("framer-motion", "Framer Motion", frontend),
        ("lucide-react", "Lucide React", frontend),
        ("recharts", "Recharts", frontend),
        ("echarts", "ECharts", frontend),
        ("vitest", "Vitest", testing),
        ("jest", "Jest", testing),
        ("@playwright/test", "Playwright", testing),
        ("cypress", "Cypress", testing),
        ("storybook", "Storybook", testing),
        ("@storybook/react", "Storybook React", testing),
        ("eslint", "ESLint", tools),
        ("prettier", "Prettier", tools),
        ("express", "Express", backend),
        ("fastify", "Fastify", backend),
        ("nestjs", "NestJS", backend),
    ]
    for package, label, target in checks:
        if package in dep_names and label not in target:
            target.append(label)

    if (root / "components.json").exists() and "shadcn/ui" not in frontend:
        frontend.append("shadcn/ui-style component registry")
    if (root / "Dockerfile").exists() or list(root.glob("Dockerfile*")):
        infrastructure.append("Dockerfile")
    if (root / "docker-compose.yml").exists() or (root / "docker-compose.yaml").exists():
        infrastructure.append("Docker Compose")
    if (root / ".github" / "workflows").exists():
        infrastructure.append("GitHub Actions")
    if "pyproject.toml" in files or "requirements.txt" in files:
        backend.append("Python")
    if "go.mod" in files:
        backend.append("Go")
    if "cargo.toml" in files:
        backend.append("Rust")

    return {
        "frontend": sorted(frontend),
        "backend": sorted(backend),
        "tools": sorted(tools),
        "testing": sorted(testing),
        "styling": sorted(styling),
        "infrastructure": sorted(infrastructure),
        "package_managers": sorted(detect_package_managers(root)),
    }


def write_key_value_lines(out: Any, mapping: dict[str, Any]) -> None:
    for key, value in mapping.items():
        out.write(f"{key:<32}: {value}\n")


def collect_basic_inventory(root: Path) -> dict[str, Any]:
    files = list(iter_project_files(root))
    dirs = list(iter_project_dirs(root))
    sizes: list[tuple[Path, int]] = []
    for path in files:
        try:
            sizes.append((path, path.stat().st_size))
        except Exception:
            sizes.append((path, 0))

    total_size = sum(size for _path, size in sizes)
    by_ext_count: Counter[str] = Counter(extension_key(path) for path in files)
    by_ext_size: Counter[str] = Counter()
    language_count: Counter[str] = Counter()
    language_size: Counter[str] = Counter()

    for path, size in sizes:
        ext = extension_key(path)
        by_ext_size[ext] += size
        language = LANGUAGE_BY_EXTENSION.get(ext)
        if language:
            language_count[language] += 1
            language_size[language] += size

    return {
        "files": files,
        "dirs": dirs,
        "sizes": sizes,
        "total_size": total_size,
        "by_ext_count": by_ext_count,
        "by_ext_size": by_ext_size,
        "language_count": language_count,
        "language_size": language_size,
        "stack": detect_stack(root),
    }
