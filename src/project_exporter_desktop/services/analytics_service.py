from __future__ import annotations

import json
import os
import re
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

from ..constants import (
    LANGUAGE_BY_EXTENSION,
    SECRET_KEY_PATTERN,
    SENSITIVE_FILENAMES,
    SENSITIVE_SUFFIXES,
    SOURCE_CODE_EXTENSIONS,
    TEXT_EXTENSIONS,
    TEXT_FILENAMES_WITHOUT_EXTENSION,
)
from ..utils.path_utils import should_ignore_dir
from ..utils.text_utils import format_bytes
from .stack_detector import detect_stack


@dataclass(slots=True)
class LanguageStat:
    name: str
    files: int = 0
    loc: int = 0
    bytes: int = 0


@dataclass(slots=True)
class DependencyItem:
    manager: str
    name: str
    version: str
    warning: str = ""


@dataclass(slots=True)
class GitCommit:
    short_hash: str
    date: str
    author: str
    subject: str


@dataclass(slots=True)
class RiskItem:
    path: str
    reason: str
    severity: str = "medium"


@dataclass(slots=True)
class AnalyticsReport:
    project_name: str
    source_root: str
    stack: str = "Не определён"
    total_files: int = 0
    total_bytes: int = 0
    total_loc: int = 0
    languages: list[LanguageStat] = field(default_factory=list)
    dependencies: list[DependencyItem] = field(default_factory=list)
    git_branch: str = ""
    git_dirty: bool = False
    git_status: str = ""
    git_commits: list[GitCommit] = field(default_factory=list)
    risks: list[RiskItem] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)

    @property
    def total_size_human(self) -> str:
        return format_bytes(self.total_bytes)


def _language_for(path: Path) -> str:
    name = path.name.casefold()
    suffix = path.suffix.casefold().lstrip(".")
    if name in {"dockerfile", "makefile"}:
        return LANGUAGE_BY_EXTENSION.get(name, name.upper())
    return LANGUAGE_BY_EXTENSION.get(suffix, "Другое")


def _is_text(path: Path) -> bool:
    suffix = path.suffix.casefold().lstrip(".")
    return suffix in TEXT_EXTENSIONS or path.name.casefold() in TEXT_FILENAMES_WITHOUT_EXTENSION


def _read_text(path: Path, limit: int = 512 * 1024) -> str:
    try:
        data = path.read_bytes()[:limit]
        return data.decode("utf-8", errors="replace")
    except Exception:
        return ""


def _count_loc(path: Path) -> int:
    if path.suffix.casefold().lstrip(".") not in SOURCE_CODE_EXTENSIONS and not _is_text(path):
        return 0
    text = _read_text(path)
    if not text:
        return 0
    return sum(1 for line in text.splitlines() if line.strip())


def _dependency_warning(version: str) -> str:
    value = version.strip()
    if not value:
        return "версия не указана"
    if value in {"*", "latest"} or "latest" in value.casefold():
        return "плавающая версия"
    if value.startswith((">", ">=", "~", "^")):
        return "диапазон версии"
    return ""


def _read_package_json(root: Path) -> list[DependencyItem]:
    path = root / "package.json"
    if not path.exists():
        return []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return []
    items: list[DependencyItem] = []
    for section in ["dependencies", "devDependencies", "peerDependencies"]:
        deps = data.get(section)
        if not isinstance(deps, dict):
            continue
        for name, version in deps.items():
            value = str(version)
            items.append(DependencyItem(f"npm/{section}", str(name), value, _dependency_warning(value)))
    return items


def _read_requirements(root: Path) -> list[DependencyItem]:
    result: list[DependencyItem] = []
    for filename in ["requirements.txt", "requirements-dev.txt"]:
        path = root / filename
        if not path.exists():
            continue
        for line in _read_text(path).splitlines():
            value = line.strip()
            if not value or value.startswith("#") or value.startswith("-"):
                continue
            match = re.match(r"([A-Za-z0-9_.-]+)\s*([=<>!~].*)?$", value)
            if not match:
                continue
            version = (match.group(2) or "").strip()
            result.append(DependencyItem(filename, match.group(1), version, _dependency_warning(version)))
    return result


def _read_go_mod(root: Path) -> list[DependencyItem]:
    path = root / "go.mod"
    if not path.exists():
        return []
    result: list[DependencyItem] = []
    for line in _read_text(path).splitlines():
        value = line.strip()
        if not value or value.startswith(("module ", "go ", "require (", ")", "//")):
            continue
        if value.startswith("require "):
            value = value.removeprefix("require ").strip()
        parts = value.split()
        if len(parts) >= 2:
            result.append(DependencyItem("go.mod", parts[0], parts[1], _dependency_warning(parts[1])))
    return result


def _read_cargo_toml(root: Path) -> list[DependencyItem]:
    path = root / "Cargo.toml"
    if not path.exists():
        return []
    result: list[DependencyItem] = []
    in_deps = False
    for line in _read_text(path).splitlines():
        value = line.strip()
        if value.startswith("["):
            in_deps = value in {"[dependencies]", "[dev-dependencies]", "[build-dependencies]"}
            continue
        if not in_deps or not value or value.startswith("#") or "=" not in value:
            continue
        name, version = value.split("=", 1)
        version_value = version.strip().strip('"')
        result.append(
            DependencyItem("Cargo.toml", name.strip(), version_value, _dependency_warning(version_value))
        )
    return result


def _git_lines(args: list[str], cwd: Path) -> tuple[int, list[str]]:
    try:
        completed = subprocess.run(
            ["git", *args],
            cwd=str(cwd),
            text=True,
            capture_output=True,
            timeout=20,
            encoding="utf-8",
            errors="replace",
        )
        return completed.returncode, completed.stdout.splitlines()
    except Exception:
        return 1, []


def _git_summary(root: Path) -> tuple[str, bool, str, list[GitCommit]]:
    rc, branch_lines = _git_lines(["branch", "--show-current"], root)
    if rc != 0:
        return "", False, "Git недоступен или проект не является репозиторием.", []
    branch = branch_lines[0].strip() if branch_lines else "detached"
    _rc, status_lines = _git_lines(["status", "--porcelain"], root)
    dirty = bool(status_lines)
    rc, commit_lines = _git_lines(
        ["log", "-10", "--date=short", "--pretty=format:%h%x09%ad%x09%an%x09%s"], root
    )
    commits: list[GitCommit] = []
    if rc == 0:
        for line in commit_lines:
            parts = line.split("\t", 3)
            if len(parts) == 4:
                commits.append(GitCommit(parts[0], parts[1], parts[2], parts[3]))
    status = "Есть незакоммиченные изменения" if dirty else "Рабочее дерево чистое"
    return branch, dirty, status, commits


def _risk_for_file(root: Path, path: Path) -> RiskItem | None:
    rel = str(path.relative_to(root)).replace("/", "\\")
    name = path.name.casefold()
    suffix = path.suffix.casefold().lstrip(".")
    if name in SENSITIVE_FILENAMES:
        return RiskItem(rel, "похоже на файл с секретами", "high")
    if suffix in SENSITIVE_SUFFIXES:
        return RiskItem(rel, "чувствительное расширение файла", "high")
    if _is_text(path):
        text = _read_text(path, limit=128 * 1024)
        if SECRET_KEY_PATTERN.search(text):
            return RiskItem(rel, "найдены ключевые слова секрета", "medium")
    return None


def analyze_project(root: Path, ignored_dirs: frozenset[str] | set[str]) -> AnalyticsReport:
    stacks = detect_stack(root)
    stack = " + ".join(item.name for item in stacks[:3]) if stacks else "Не определён"
    report = AnalyticsReport(project_name=root.name, source_root=str(root), stack=stack)
    by_language: dict[str, LanguageStat] = {}

    for current_dir, dirnames, filenames in os.walk(root, topdown=True, followlinks=False):
        current = Path(current_dir)
        dirnames[:] = [
            dirname
            for dirname in dirnames
            if not should_ignore_dir(dirname, ignored_dirs) and not (current / dirname).is_symlink()
        ]
        for filename in filenames:
            path = current / filename
            if path.is_symlink() or not path.is_file():
                continue
            try:
                size = path.stat().st_size
            except Exception:
                size = 0
            language = _language_for(path)
            loc = _count_loc(path)
            stat = by_language.setdefault(language, LanguageStat(language))
            stat.files += 1
            stat.bytes += size
            stat.loc += loc
            report.total_files += 1
            report.total_bytes += size
            report.total_loc += loc
            risk = _risk_for_file(root, path)
            if risk is not None and len(report.risks) < 100:
                report.risks.append(risk)

    report.languages = sorted(by_language.values(), key=lambda item: item.loc, reverse=True)
    report.dependencies = (
        _read_package_json(root)
        + _read_requirements(root)
        + _read_go_mod(root)
        + _read_cargo_toml(root)
    )
    report.git_branch, report.git_dirty, report.git_status, report.git_commits = _git_summary(root)
    return report
