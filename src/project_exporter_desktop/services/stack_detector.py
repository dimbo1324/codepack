from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path


@dataclass(frozen=True)
class StackInfo:
    """Описание обнаруженного стека проекта."""

    name: str
    markers_found: tuple[str, ...]
    extra_ignored_dirs: frozenset[str] = field(default_factory=frozenset)

    def display_label(self) -> str:
        """Строка для отображения в UI."""
        found = ", ".join(self.markers_found[:3])
        return f"Обнаружен стек: {self.name}  ({found})"


_STACK_RULES: list[dict] = [
    {
        "name": "Node.js",
        "markers": ["package.json"],
        "ignored_dirs": {
            "node_modules",
            ".pnp",
            ".npm",
            ".yarn",
            ".next",
            ".nuxt",
            ".expo",
            ".turbo",
            ".parcel-cache",
            "out",
        },
    },
    {
        "name": "Python",
        "markers": ["requirements.txt", "pyproject.toml", "setup.py", "setup.cfg", "Pipfile"],
        "ignored_dirs": {
            ".venv",
            "venv",
            "env",
            "__pycache__",
            ".mypy_cache",
            ".ruff_cache",
            ".pytest_cache",
            ".tox",
            "htmlcov",
            "*.egg-info",
        },
    },
    {
        "name": "Go",
        "markers": ["go.mod"],
        "ignored_dirs": {"vendor"},
    },
    {
        "name": "Rust",
        "markers": ["Cargo.toml"],
        "ignored_dirs": {"target"},
    },
    {
        "name": "Java / Maven",
        "markers": ["pom.xml"],
        "ignored_dirs": {"target", ".mvn"},
    },
    {
        "name": "Java / Gradle",
        "markers": ["build.gradle", "settings.gradle", "build.gradle.kts"],
        "ignored_dirs": {"build", ".gradle", ".idea"},
    },
    {
        "name": ".NET / C#",
        "markers": [],
        "marker_extensions": [".csproj", ".sln"],
        "ignored_dirs": {"bin", "obj", ".vs"},
    },
    {
        "name": "Flutter / Dart",
        "markers": ["pubspec.yaml"],
        "ignored_dirs": {".dart_tool", "build", ".flutter-plugins"},
    },
    {
        "name": "PHP / Composer",
        "markers": ["composer.json"],
        "ignored_dirs": {"vendor"},
    },
    {
        "name": "Ruby",
        "markers": ["Gemfile"],
        "ignored_dirs": {".bundle", "vendor/bundle"},
    },
    {
        "name": "iOS / Swift",
        "markers": ["Package.swift"],
        "marker_extensions": [".xcodeproj", ".xcworkspace"],
        "ignored_dirs": {".build", "DerivedData"},
    },
    {
        "name": "Android",
        "markers": ["AndroidManifest.xml"],
        "ignored_dirs": {"build", ".gradle", ".idea"},
    },
]


def detect_stack(root: Path) -> list[StackInfo]:
    """
    Определяет стек(и) проекта по маркерным файлам в корневой папке.

    Возвращает список StackInfo от наиболее к наименее очевидному.
    Пустой список означает неизвестный стек.
    """
    if not root.is_dir():
        return []

    try:
        root_names = {entry.name for entry in root.iterdir() if not entry.name.startswith(".")}
        root_names_with_dot = {entry.name for entry in root.iterdir()}
        root_suffixes = {entry.suffix for entry in root.iterdir() if entry.is_file()}
    except PermissionError:
        return []

    results: list[StackInfo] = []
    for rule in _STACK_RULES:
        markers: list[str] = rule.get("markers", [])
        marker_extensions: list[str] = rule.get("marker_extensions", [])
        ignored_dirs: set[str] = rule.get("ignored_dirs", set())

        found: list[str] = []
        for marker in markers:
            if marker in root_names_with_dot:
                found.append(marker)

        for ext in marker_extensions:
            if ext in root_suffixes:
                found.append(f"*{ext}")

        if found:
            results.append(
                StackInfo(
                    name=rule["name"],
                    markers_found=tuple(found),
                    extra_ignored_dirs=frozenset(ignored_dirs),
                )
            )

    results.sort(key=lambda s: len(s.markers_found), reverse=True)
    return results


def primary_stack(root: Path) -> StackInfo | None:
    """Возвращает основной (наиболее очевидный) стек или None."""
    stacks = detect_stack(root)
    return stacks[0] if stacks else None


def merged_extra_ignored_dirs(root: Path) -> frozenset[str]:
    """Объединяет ignore-директории всех обнаруженных стеков."""
    result: set[str] = set()
    for info in detect_stack(root):
        result |= info.extra_ignored_dirs
    return frozenset(result)


def format_stack_label(root: Path) -> str:
    """Строка для отображения в UI страницы проекта."""
    stacks = detect_stack(root)
    if not stacks:
        return ""
    if len(stacks) == 1:
        return stacks[0].display_label()
    names = " + ".join(s.name for s in stacks[:3])
    markers = ", ".join(m for s in stacks[:3] for m in s.markers_found[:2])
    return f"Обнаружен стек: {names}  ({markers})"
