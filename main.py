#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Project Exporter Desktop — single-file Windows desktop utility.

Produces ONE final ZIP archive on the user's Desktop containing:

    {project}_export_{timestamp}.zip
    ├── INDEX.md                            human-readable bundle TOC
    ├── manifest.json                       machine-readable metadata
    ├── {project}/                          working copy of the project
    │   └── ...                             (without .git / node_modules,
    │                                       symlinks, and any user-defined
    │                                       extra ignored directories)
    └── reports/
        ├── 01_structure.txt                PowerShell-like directory listing
        ├── 02_git.txt                      read-only Git inspection
        ├── 03_text_dump.txt                concatenated text-file contents
        └── insights/
            ├── 01_summary.txt
            ├── 02_file_statistics.txt
            ├── 03_dependencies.txt
            ├── 04_scripts.txt
            ├── 05_git_deep.txt
            ├── 06_security_scan.txt
            ├── 07_todo_fixme.txt
            ├── 08_code_metrics.txt
            ├── 09_config.txt
            ├── 10_docker.txt
            ├── 11_routes_and_pages.txt
            └── 12_ai_context_pack.md

Business logic preserved from the previous version:
- Always exclude .git and node_modules from the copy. The user may add
  extra directories on top of these defaults — never replace them.
- Read-only Git commands only; the original repository is never modified
  (no branch switch, no checkout, no fetch).
- Symbolic links are skipped to prevent accidental escape from the tree.
- Secrets in the text dump can be redacted by simple heuristics.

Python: 3.14+
Dependencies: standard library only.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import threading
import traceback
import zipfile
from collections import Counter, defaultdict
from collections.abc import Callable, Iterable
from dataclasses import dataclass, asdict, field
from datetime import datetime
from pathlib import Path
from queue import Empty, Queue
from typing import Any

import tkinter as tk
from tkinter import filedialog, messagebox, scrolledtext, ttk

# ---------------------------------------------------------------------------
# Application identity
# ---------------------------------------------------------------------------

APP_NAME = "Project Exporter Desktop"
APP_VERSION = "2.0"
SETTINGS_FILE = Path.home() / ".project_exporter_desktop.json"


# ---------------------------------------------------------------------------
# Default ignored directories (business logic — never replaced, only extended)
# ---------------------------------------------------------------------------

IGNORED_DIR_NAMES: frozenset[str] = frozenset(
    {
        ".git",
        "node_modules",
    }
)


# ---------------------------------------------------------------------------
# Text / binary detection sets
# ---------------------------------------------------------------------------

TEXT_EXTENSIONS: set[str] = {
    "adoc",
    "ahk",
    "asciidoc",
    "astro",
    "bash",
    "bat",
    "bib",
    "bzl",
    "c",
    "cc",
    "cfg",
    "cjs",
    "clj",
    "cljc",
    "cljs",
    "cls",
    "cmake",
    "cmd",
    "conf",
    "config",
    "cpp",
    "cs",
    "css",
    "csv",
    "cxx",
    "d",
    "dart",
    "desktop",
    "diff",
    "dockerfile",
    "dockerignore",
    "editorconfig",
    "edn",
    "ejs",
    "elm",
    "env",
    "erl",
    "err",
    "ex",
    "exs",
    "feature",
    "fish",
    "fs",
    "fsi",
    "fsx",
    "gemspec",
    "gitattributes",
    "gitignore",
    "go",
    "gql",
    "gradle",
    "graphql",
    "groovy",
    "h",
    "haml",
    "hcl",
    "hh",
    "hpp",
    "hrl",
    "hs",
    "htm",
    "html",
    "hxx",
    "ini",
    "ipynb",
    "java",
    "jl",
    "js",
    "json",
    "json5",
    "jsx",
    "kt",
    "kts",
    "less",
    "lhs",
    "lock",
    "log",
    "lua",
    "makefile",
    "md",
    "markdown",
    "mjs",
    "mk",
    "ml",
    "mli",
    "nim",
    "npmrc",
    "nvmrc",
    "odin",
    "org",
    "out",
    "patch",
    "php",
    "phtml",
    "pl",
    "pm",
    "pod",
    "properties",
    "proto",
    "ps1",
    "psm1",
    "py",
    "pyi",
    "pyw",
    "r",
    "rake",
    "rb",
    "rs",
    "rst",
    "sass",
    "scala",
    "scss",
    "sh",
    "sol",
    "sql",
    "svelte",
    "tex",
    "tf",
    "tfvars",
    "toml",
    "ts",
    "tsv",
    "tsx",
    "txt",
    "v",
    "vb",
    "vbs",
    "vue",
    "xml",
    "yaml",
    "yml",
    "zig",
    "zsh",
}

TEXT_FILENAMES_WITHOUT_EXTENSION: set[str] = {
    ".env",
    ".env.example",
    ".env.local",
    ".env.development",
    ".env.production",
    ".gitignore",
    ".gitattributes",
    ".dockerignore",
    ".editorconfig",
    ".npmrc",
    ".nvmrc",
    "dockerfile",
    "makefile",
    "readme",
    "license",
}

BINARY_EXTENSIONS: set[str] = {
    "7z",
    "a",
    "aac",
    "accdb",
    "ai",
    "aiff",
    "apk",
    "avi",
    "bin",
    "blend",
    "bmp",
    "bz2",
    "cab",
    "class",
    "db",
    "dll",
    "dmg",
    "doc",
    "docx",
    "dwg",
    "dylib",
    "ear",
    "eot",
    "epub",
    "exe",
    "fbx",
    "flac",
    "flv",
    "gif",
    "gz",
    "heic",
    "ico",
    "iso",
    "jar",
    "jpeg",
    "jpg",
    "lib",
    "m4a",
    "m4v",
    "max",
    "mdb",
    "mkv",
    "mov",
    "mp3",
    "mp4",
    "mpeg",
    "mpg",
    "o",
    "obj",
    "odp",
    "ods",
    "odt",
    "ogg",
    "opus",
    "otf",
    "pdf",
    "png",
    "ppt",
    "pptx",
    "psd",
    "pyc",
    "pyd",
    "pyo",
    "rar",
    "raw",
    "so",
    "sqlite",
    "sqlite3",
    "tar",
    "tif",
    "tiff",
    "ttf",
    "wav",
    "war",
    "webm",
    "webp",
    "wmv",
    "woff",
    "woff2",
    "wma",
    "xls",
    "xlsx",
    "xz",
    "zip",
}

TRY_ENCODINGS: tuple[str, ...] = (
    "utf-8",
    "utf-8-sig",
    "cp1251",
    "cp866",
    "utf-16",
    "latin-1",
)


# ---------------------------------------------------------------------------
# Secret detection patterns (single source of truth)
# ---------------------------------------------------------------------------

# Keywords used to redact `KEY = value` style assignments in the text dump.
_REDACT_KEYWORDS = r"API[_-]?KEY|SECRET|TOKEN|PASSWORD|PASS|PRIVATE[_-]?KEY"

# Extended keyword set used by the security scan report (flagging only,
# no redaction by itself; pairs with the redact patterns).
_SCAN_KEYWORDS = (
    _REDACT_KEYWORDS
    + r"|DATABASE[_-]?URL|JWT[_-]?SECRET|ACCESS[_-]?KEY|CLIENT[_-]?SECRET"
)

SECRET_PATTERNS: tuple[re.Pattern[str], ...] = (
    re.compile(rf"(?i)\b({_REDACT_KEYWORDS})\b\s*[:=]\s*(['\"]?)[^\s'\"\n]+(\2)"),
    re.compile(r"(?i)\b(BEARER)\s+[A-Za-z0-9._\-+/=]{16,}"),
)

SECRET_KEY_PATTERN = re.compile(rf"(?i)\b({_SCAN_KEYWORDS})\b")

TODO_PATTERN = re.compile(r"(?i)\b(TODO|FIXME|HACK|XXX|BUG|TEMP|REFACTOR|DEPRECATED)\b")


# ---------------------------------------------------------------------------
# Persistent user configuration
# ---------------------------------------------------------------------------


@dataclass(slots=True)
class Config:
    last_root: str = str(Path.home())
    max_text_file_mb: int = 5
    redact_secrets: bool = True
    keep_staging_folder: bool = False
    include_project_in_zip: bool = True
    extra_ignored_dirs: list[str] = field(default_factory=list)

    @classmethod
    def load(cls) -> Config:
        try:
            if SETTINGS_FILE.exists():
                data = json.loads(SETTINGS_FILE.read_text(encoding="utf-8"))
                # Tolerate older settings files that lack the new fields.
                known = {f.name for f in cls.__dataclass_fields__.values()}
                data = {k: v for k, v in data.items() if k in known}
                return cls(**data)
        except Exception:
            return cls()
        return cls()

    def save(self) -> None:
        try:
            SETTINGS_FILE.write_text(
                json.dumps(asdict(self), ensure_ascii=False, indent=2),
                encoding="utf-8",
            )
        except Exception:
            pass

    def effective_ignored_dirs(self) -> frozenset[str]:
        """Defaults are always present; user values are additive only."""
        extras = {name.strip() for name in self.extra_ignored_dirs if name.strip()}
        return IGNORED_DIR_NAMES | extras


# ---------------------------------------------------------------------------
# Bundle path layout
# ---------------------------------------------------------------------------


@dataclass(slots=True)
class ExportPaths:
    desktop: Path
    source_root: Path
    project_name: str
    bundle_name: str  # "{project}_export_{timestamp}"
    staging_dir: Path  # ~/Desktop/{bundle_name}
    final_zip: Path  # ~/Desktop/{bundle_name}.zip
    project_dir: Path  # {staging_dir}/{project_name}
    reports_dir: Path  # {staging_dir}/reports
    insights_dir: Path  # {staging_dir}/reports/insights
    manifest_file: Path  # {staging_dir}/manifest.json
    index_file: Path  # {staging_dir}/INDEX.md
    structure_report: Path  # {reports_dir}/01_structure.txt
    git_report: Path  # {reports_dir}/02_git.txt
    text_dump: Path  # {reports_dir}/03_text_dump.txt


@dataclass(slots=True)
class CopyStats:
    dirs_created: int = 0
    files_copied: int = 0
    dirs_skipped: int = 0
    files_skipped: int = 0
    symlinks_skipped: int = 0
    errors: int = 0


@dataclass(slots=True)
class TextDumpStats:
    scanned: int = 0
    written: int = 0
    skipped_binary: int = 0
    skipped_large: int = 0
    skipped_decode: int = 0
    skipped_not_text: int = 0


# ---------------------------------------------------------------------------
# Small utilities
# ---------------------------------------------------------------------------


def now_stamp() -> str:
    return datetime.now().strftime("%Y%m%d_%H%M%S")


def human_now() -> str:
    return datetime.now().isoformat(sep=" ", timespec="seconds")


def desktop_path() -> Path:
    desktop = Path.home() / "Desktop"
    return desktop if desktop.exists() else Path.home()


def sanitize_name(name: str) -> str:
    cleaned = "".join(ch for ch in name if ch not in '<>:"/\\|?*').strip()
    return cleaned or "project"


def is_relative_to(child: Path, parent: Path) -> bool:
    try:
        child.resolve().relative_to(parent.resolve())
        return True
    except ValueError:
        return False


def validate_source_root(path_text: str) -> Path:
    if not path_text.strip():
        raise ValueError("Укажите корневую папку проекта.")

    root = Path(path_text).expanduser().resolve()

    if not root.exists():
        raise ValueError("Указанная папка не существует.")
    if not root.is_dir():
        raise ValueError("Указанный путь не является папкой.")
    if root.parent == root:
        raise ValueError("Нельзя выбирать корень диска.")
    if root == Path.home().resolve():
        raise ValueError(
            "Не выбирайте всю домашнюю папку целиком. Укажите конкретный проект."
        )

    return root


def build_export_paths(source_root: Path) -> ExportPaths:
    """Allocate a unique bundle path. If a collision is detected, suffix it."""
    desktop = desktop_path()
    project_name = sanitize_name(source_root.name)

    base = f"{project_name}_export_{now_stamp()}"
    bundle_name = base
    staging = desktop / bundle_name
    final_zip = desktop / f"{bundle_name}.zip"

    counter = 1
    while staging.exists() or final_zip.exists():
        bundle_name = f"{base}_{counter}"
        staging = desktop / bundle_name
        final_zip = desktop / f"{bundle_name}.zip"
        counter += 1

    reports_dir = staging / "reports"
    insights_dir = reports_dir / "insights"

    return ExportPaths(
        desktop=desktop,
        source_root=source_root,
        project_name=project_name,
        bundle_name=bundle_name,
        staging_dir=staging,
        final_zip=final_zip,
        project_dir=staging / project_name,
        reports_dir=reports_dir,
        insights_dir=insights_dir,
        manifest_file=staging / "manifest.json",
        index_file=staging / "INDEX.md",
        structure_report=reports_dir / "01_structure.txt",
        git_report=reports_dir / "02_git.txt",
        text_dump=reports_dir / "03_text_dump.txt",
    )


def should_ignore_dir(
    name: str, extra: frozenset[str] | set[str] = frozenset()
) -> bool:
    return name in IGNORED_DIR_NAMES or name in extra


def rel_display(path: Path, root: Path) -> str:
    rel = path.relative_to(root)
    if str(rel) == ".":
        return "."
    return ".\\" + str(rel).replace("/", "\\")


def ps_mode(path: Path) -> str:
    return "d-----" if path.is_dir() else "-a----"


MONTHS = (
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
)


def ps_date(timestamp: float) -> str:
    dt = datetime.fromtimestamp(timestamp)
    month = MONTHS[dt.month - 1]
    return f"{dt.day:02d}-{month}-{dt.year % 100:02d}     {dt.hour:02d}:{dt.minute:02d}"


# ---------------------------------------------------------------------------
# Step 1: project copy
# ---------------------------------------------------------------------------


def copy_project(
    source_root: Path,
    destination_root: Path,
    extra_ignored_dirs: frozenset[str] | set[str],
    log: Callable[[str], None],
    cancel: threading.Event,
) -> CopyStats:
    stats = CopyStats()
    destination_root.parent.mkdir(parents=True, exist_ok=True)

    log(f"Создаю копию проекта: {destination_root}")

    for current_dir, dirnames, filenames in os.walk(
        source_root, topdown=True, followlinks=False
    ):
        if cancel.is_set():
            log("Копирование остановлено пользователем.")
            break

        current = Path(current_dir)

        safe_dirnames: list[str] = []
        for dirname in dirnames:
            if should_ignore_dir(dirname, extra_ignored_dirs):
                stats.dirs_skipped += 1
                log(f"Пропущена папка: {rel_display(current / dirname, source_root)}")
                continue

            child = current / dirname
            if child.is_symlink():
                stats.symlinks_skipped += 1
                log(
                    f"Пропущена символическая ссылка на папку: "
                    f"{rel_display(child, source_root)}"
                )
                continue

            safe_dirnames.append(dirname)

        dirnames[:] = safe_dirnames

        relative_dir = current.relative_to(source_root)
        target_dir = destination_root / relative_dir
        try:
            target_dir.mkdir(parents=True, exist_ok=True)
            stats.dirs_created += 1
        except Exception as exc:
            stats.errors += 1
            log(f"Ошибка создания папки {target_dir}: {exc}")
            continue

        for filename in filenames:
            if cancel.is_set():
                break

            src_file = current / filename
            if src_file.is_symlink():
                stats.symlinks_skipped += 1
                log(
                    f"Пропущена символическая ссылка на файл: "
                    f"{rel_display(src_file, source_root)}"
                )
                continue

            dst_file = target_dir / filename

            try:
                shutil.copy2(src_file, dst_file)
                stats.files_copied += 1
                if stats.files_copied % 250 == 0:
                    log(f"Скопировано файлов: {stats.files_copied:,}")
            except Exception as exc:
                stats.errors += 1
                log(f"Ошибка копирования {rel_display(src_file, source_root)}: {exc}")

    return stats


# ---------------------------------------------------------------------------
# Final bundle zip (replaces the per-project zip)
# ---------------------------------------------------------------------------


def build_final_zip(
    paths: ExportPaths,
    include_project: bool,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> int:
    """Pack the entire staging directory into a single ZIP on the Desktop.

    The archive layout matches the staging directory: INDEX.md, manifest.json,
    {project}/ (optional), and reports/. When ``include_project`` is False,
    the {project}/ subtree is skipped — the archive then contains only the
    reports bundle, which is useful when only an overview is needed.
    """
    log(f"Создаю итоговый ZIP-архив: {paths.final_zip.name}")
    file_count = 0
    skipped_project_files = 0

    project_dir_resolved = paths.project_dir.resolve()
    staging_resolved = paths.staging_dir.resolve()

    with zipfile.ZipFile(
        paths.final_zip, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=6
    ) as archive:
        for file_path in paths.staging_dir.rglob("*"):
            if cancel.is_set():
                log("Создание архива остановлено пользователем.")
                break
            if not file_path.is_file():
                continue

            resolved = file_path.resolve()
            if not include_project:
                try:
                    resolved.relative_to(project_dir_resolved)
                    skipped_project_files += 1
                    continue
                except ValueError:
                    pass  # Outside the project subtree — keep it.

            try:
                arcname = resolved.relative_to(staging_resolved)
            except ValueError:
                # Should not happen, but stay defensive.
                arcname = Path(file_path.name)

            archive.write(file_path, arcname)
            file_count += 1
            if file_count % 500 == 0:
                log(f"Добавлено в ZIP: {file_count:,} файлов")

    if skipped_project_files:
        log(
            f"Копия проекта исключена из ZIP по настройке "
            f"({skipped_project_files:,} файлов)"
        )
    log(f"ZIP готов: {file_count:,} файлов → {paths.final_zip}")
    return file_count


# ---------------------------------------------------------------------------
# Step 2: directory structure report
# ---------------------------------------------------------------------------


def write_structure_report(
    root: Path,
    output_file: Path,
    extra_ignored_dirs: frozenset[str] | set[str],
    log: Callable[[str], None],
    cancel: threading.Event,
) -> int:
    log(f"Формирую отчёт структуры: {output_file.name}")
    groups_written = 0

    ignored_display = ", ".join(sorted(IGNORED_DIR_NAMES | set(extra_ignored_dirs)))

    with output_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write("=== Relative Project Structure ===\n")
        out.write(f"Project copy root name: {root.name}\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(f"Ignored directories: {ignored_display}\n")
        out.write("=" * 100 + "\n\n")

        for current_dir, dirnames, filenames in os.walk(
            root, topdown=True, followlinks=False
        ):
            if cancel.is_set():
                break

            current = Path(current_dir)
            dirnames[:] = [
                d for d in dirnames if not should_ignore_dir(d, extra_ignored_dirs)
            ]

            entries: list[Path] = []
            for dirname in sorted(dirnames, key=str.lower):
                entries.append(current / dirname)
            for filename in sorted(filenames, key=str.lower):
                entries.append(current / filename)

            if not entries:
                continue

            out.write(f"    Directory: {rel_display(current, root)}\n\n")
            out.write(f"{'Mode':<20} {'LastWriteTime':<20} {'Length':>12} Name\n")
            out.write(f"{'----':<20} {'-------------':<20} {'------':>12} ----\n")

            for entry in entries:
                try:
                    stat = entry.stat()
                    length = "" if entry.is_dir() else str(stat.st_size)
                    out.write(
                        f"{ps_mode(entry):<20} {ps_date(stat.st_mtime):<20} "
                        f"{length:>12} {entry.name}\n"
                    )
                except Exception as exc:
                    out.write(f"{'ERROR':<20} {'':<20} {'':>12} {entry.name} ({exc})\n")

            out.write("\n\n")
            groups_written += 1

    log(f"Отчёт структуры готов: {groups_written:,} директорий")
    return groups_written


# ---------------------------------------------------------------------------
# Step 3: Git inspection (read-only)
# ---------------------------------------------------------------------------


def run_git_command(
    args: list[str], cwd: Path, timeout_seconds: int = 120
) -> tuple[int | None, str, str]:
    try:
        completed = subprocess.run(
            args,
            cwd=str(cwd),
            text=True,
            capture_output=True,
            timeout=timeout_seconds,
            shell=False,
            encoding="utf-8",
            errors="replace",
        )
        return completed.returncode, completed.stdout, completed.stderr
    except subprocess.TimeoutExpired as exc:
        stdout = exc.stdout if isinstance(exc.stdout, str) else ""
        stderr = exc.stderr if isinstance(exc.stderr, str) else ""
        return None, stdout, stderr + f"\nTIMEOUT after {timeout_seconds} seconds."
    except FileNotFoundError:
        return (
            None,
            "",
            "Git executable was not found. Install Git for Windows and "
            "ensure git.exe is in PATH.",
        )
    except Exception as exc:
        return None, "", f"{type(exc).__name__}: {exc}"


def write_git_report(
    source_root: Path,
    output_file: Path,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> None:
    log(f"Формирую Git-отчёт: {output_file.name}")

    commands: list[list[str]] = [
        ["git", "status", "--short", "--branch"],
        ["git", "branch", "--show-current"],
        ["git", "log", "--oneline", "-5"],
        ["git", "show", "--stat", "HEAD"],
        ["git", "show", "HEAD"],
    ]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Git Report ===\n")
        out.write(f"Source root: {source_root}\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(
            "Note: Git data is collected from the ORIGINAL project (the .git "
            "directory is intentionally never copied into the bundle).\n"
        )
        out.write("=" * 100 + "\n\n")

        git_dir = source_root / ".git"
        if not git_dir.exists():
            out.write("No .git directory was found in the selected root.\n")
            out.write("Git commands were not executed.\n")
            log("Git-папка не найдена. Git-команды пропущены.")
            return

        for command in commands:
            if cancel.is_set():
                out.write("\nOperation cancelled by user.\n")
                break

            command_text = " ".join(command)
            log(f"Выполняю: {command_text}")

            out.write("\n" + "=" * 100 + "\n")
            out.write(f"$ {command_text}\n")
            out.write("=" * 100 + "\n\n")

            code, stdout, stderr = run_git_command(command, cwd=source_root)
            out.write(f"Exit code: {code}\n\n")

            out.write("--- STDOUT ---\n")
            out.write(stdout or "")
            if stdout and not stdout.endswith("\n"):
                out.write("\n")

            out.write("\n--- STDERR ---\n")
            out.write(stderr or "")
            if stderr and not stderr.endswith("\n"):
                out.write("\n")

            out.write("\n")

    log("Git-отчёт готов")


# ---------------------------------------------------------------------------
# Step 4: text-content dump
# ---------------------------------------------------------------------------


def looks_binary(raw: bytes) -> bool:
    if b"\x00" in raw[:8192]:
        return True

    sample = raw[:8192]
    if not sample:
        return False

    control_bytes = sum(1 for b in sample if b < 9 or (13 < b < 32))
    return (control_bytes / len(sample)) > 0.30


def should_consider_text_file(path: Path) -> bool:
    name = path.name.lower()
    suffix = path.suffix.lower().lstrip(".")

    if name in TEXT_FILENAMES_WITHOUT_EXTENSION:
        return True
    if suffix in BINARY_EXTENSIONS:
        return False
    if suffix in TEXT_EXTENSIONS:
        return True

    return False


def read_text_safely(path: Path, max_bytes: int) -> tuple[str | None, str]:
    try:
        raw = path.read_bytes()
    except Exception as exc:
        return None, f"IOError: {exc}"

    if len(raw) == 0:
        return "", "empty"
    if len(raw) > max_bytes:
        return None, f"too-large:{len(raw)}"

    if looks_binary(raw):
        return None, "binary-detected"

    for encoding in TRY_ENCODINGS:
        try:
            return raw.decode(encoding), encoding
        except UnicodeDecodeError:
            continue
        except Exception as exc:
            return None, f"DecodeError:{exc}"

    try:
        return raw.decode("latin-1", errors="replace"), "latin-1(replace)"
    except Exception as exc:
        return None, f"DecodeError:{exc}"


def redact_secrets(text: str) -> str:
    redacted = text
    for pattern in SECRET_PATTERNS:

        def repl(match: re.Match[str]) -> str:
            original = match.group(0)
            if "=" in original:
                key = original.split("=", 1)[0]
                return f"{key}=<REDACTED>"
            if ":" in original:
                key = original.split(":", 1)[0]
                return f"{key}: <REDACTED>"
            return "<REDACTED_SECRET>"

        redacted = pattern.sub(repl, redacted)
    return redacted


def write_text_dump(
    root: Path,
    output_file: Path,
    max_bytes_per_file: int,
    redact: bool,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> TextDumpStats:
    log(f"Собираю текстовое содержимое файлов: {output_file.name}")

    stats = TextDumpStats()
    files = sorted(
        (p for p in root.rglob("*") if p.is_file()), key=lambda p: str(p).lower()
    )

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Text Files Dump ===\n")
        out.write(f"Project copy root name: {root.name}\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(f"Max bytes per file: {max_bytes_per_file:,}\n")
        out.write(f"Secrets redaction: {'enabled' if redact else 'disabled'}\n")
        out.write("Only readable text-like files are included.\n")
        out.write("=" * 100 + "\n\n")

        for path in files:
            if cancel.is_set():
                log("Сбор текстовых файлов остановлен пользователем.")
                break

            stats.scanned += 1

            if not should_consider_text_file(path):
                stats.skipped_not_text += 1
                continue

            try:
                size = path.stat().st_size
            except Exception:
                stats.skipped_decode += 1
                continue

            if size > max_bytes_per_file:
                stats.skipped_large += 1
                continue

            text, info = read_text_safely(path, max_bytes=max_bytes_per_file)
            if text is None:
                if info == "binary-detected":
                    stats.skipped_binary += 1
                elif info.startswith("too-large"):
                    stats.skipped_large += 1
                else:
                    stats.skipped_decode += 1
                continue

            if redact:
                text = redact_secrets(text)

            stat = path.stat()
            out.write("\n" + "=" * 120 + "\n")
            out.write(f"File: {rel_display(path, root)}\n")
            out.write(f"Name: {path.name}\n")
            out.write(f"Size: {stat.st_size:,} bytes\n")
            out.write(
                f"Modified: "
                f"{datetime.fromtimestamp(stat.st_mtime).isoformat(sep=' ', timespec='seconds')}\n"
            )
            out.write(f"Encoding: {info}\n")
            out.write("=" * 120 + "\n\n")
            out.write(text)
            if not text.endswith("\n"):
                out.write("\n")

            stats.written += 1
            if stats.written % 50 == 0:
                log(f"Записано текстовых файлов: {stats.written:,}")

    log(f"Текстовый дамп готов: {stats.written:,} файлов")
    return stats


# Extended project insight reports
# ---------------------------------------------------------------------------

LANGUAGE_BY_EXTENSION: dict[str, str] = {
    "py": "Python",
    "pyw": "Python",
    "pyi": "Python",
    "js": "JavaScript",
    "jsx": "JavaScript / React",
    "mjs": "JavaScript",
    "cjs": "JavaScript",
    "ts": "TypeScript",
    "tsx": "TypeScript / React",
    "css": "CSS",
    "scss": "SCSS",
    "sass": "Sass",
    "less": "Less",
    "html": "HTML",
    "htm": "HTML",
    "vue": "Vue",
    "svelte": "Svelte",
    "astro": "Astro",
    "go": "Go",
    "rs": "Rust",
    "java": "Java",
    "kt": "Kotlin",
    "kts": "Kotlin",
    "cs": "C#",
    "cpp": "C++",
    "cxx": "C++",
    "cc": "C++",
    "c": "C",
    "h": "C/C++ Header",
    "hpp": "C++ Header",
    "rb": "Ruby",
    "php": "PHP",
    "swift": "Swift",
    "dart": "Dart",
    "sql": "SQL",
    "sh": "Shell",
    "bash": "Shell",
    "zsh": "Shell",
    "fish": "Shell",
    "ps1": "PowerShell",
    "bat": "Batch",
    "cmd": "Batch",
    "json": "JSON",
    "json5": "JSON5",
    "yaml": "YAML",
    "yml": "YAML",
    "toml": "TOML",
    "xml": "XML",
    "md": "Markdown",
    "markdown": "Markdown",
    "rst": "reStructuredText",
    "dockerfile": "Dockerfile",
}

SOURCE_CODE_EXTENSIONS: set[str] = {
    "astro",
    "c",
    "cc",
    "cpp",
    "cs",
    "css",
    "cxx",
    "dart",
    "go",
    "h",
    "hpp",
    "html",
    "htm",
    "java",
    "js",
    "jsx",
    "kt",
    "kts",
    "less",
    "mjs",
    "php",
    "py",
    "pyi",
    "pyw",
    "rb",
    "rs",
    "sass",
    "scss",
    "sh",
    "sql",
    "svelte",
    "ts",
    "tsx",
    "vue",
}

CONFIG_FILES: tuple[str, ...] = (
    "package.json",
    "pnpm-lock.yaml",
    "package-lock.json",
    "yarn.lock",
    "bun.lockb",
    "tsconfig.json",
    "jsconfig.json",
    "vite.config.ts",
    "vite.config.js",
    "next.config.js",
    "next.config.mjs",
    "next.config.ts",
    "eslint.config.js",
    "eslint.config.mjs",
    ".eslintrc",
    ".eslintrc.js",
    ".eslintrc.json",
    ".prettierrc",
    ".prettierrc.json",
    "prettier.config.js",
    "tailwind.config.js",
    "tailwind.config.ts",
    "postcss.config.js",
    "components.json",
    "pyproject.toml",
    "requirements.txt",
    "requirements-dev.txt",
    "poetry.lock",
    "Pipfile",
    "Pipfile.lock",
    "go.mod",
    "go.sum",
    "Cargo.toml",
    "Cargo.lock",
    "Dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    ".env.example",
    ".gitignore",
    "README.md",
    "LICENSE",
)

SENSITIVE_FILENAMES: set[str] = {
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".env.test",
    "id_rsa",
    "id_dsa",
    "id_ecdsa",
    "id_ed25519",
    "credentials.json",
    "service-account.json",
    "service_account.json",
    "firebase-adminsdk.json",
}

SENSITIVE_SUFFIXES: set[str] = {
    "key",
    "pem",
    "p12",
    "pfx",
    "crt",
    "cer",
    "keystore",
    "jks",
}

# SECRET_KEY_PATTERN and TODO_PATTERN are defined in the head section above
# as part of the unified secret-detection setup.


def format_bytes(size: int) -> str:
    units = ("B", "KB", "MB", "GB", "TB")
    value = float(size)
    for unit in units:
        if value < 1024 or unit == units[-1]:
            if unit == "B":
                return f"{int(value)} {unit}"
            return f"{value:.2f} {unit}"
        value /= 1024
    return f"{size} B"


def safe_read_json(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return {}


def iter_project_files(root: Path) -> Iterable[Path]:
    for current_dir, dirnames, filenames in os.walk(
        root, topdown=True, followlinks=False
    ):
        dirnames[:] = [
            dirname
            for dirname in dirnames
            if not should_ignore_dir(dirname)
            and not (Path(current_dir) / dirname).is_symlink()
        ]
        for filename in filenames:
            path = Path(current_dir) / filename
            if path.is_symlink():
                continue
            yield path


def iter_project_dirs(root: Path) -> Iterable[Path]:
    for current_dir, dirnames, _filenames in os.walk(
        root, topdown=True, followlinks=False
    ):
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
    files = (
        {p.name.lower() for p in root.iterdir() if p.exists()}
        if root.exists()
        else set()
    )

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
    if (root / "docker-compose.yml").exists() or (
        root / "docker-compose.yaml"
    ).exists():
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


def write_project_summary_report(
    copied_root: Path,
    source_root: Path,
    output_file: Path,
    inventory: dict[str, Any],
) -> None:
    files: list[Path] = inventory["files"]
    dirs: list[Path] = inventory["dirs"]
    sizes: list[tuple[Path, int]] = inventory["sizes"]
    stack: dict[str, list[str]] = inventory["stack"]

    readmes = [p for p in files if p.name.lower().startswith("readme")]
    licenses = [p for p in files if p.name.lower().startswith("license")]
    env_files = [p for p in files if p.name.lower().startswith(".env")]
    test_files = [
        p
        for p in files
        if re.search(
            r"(?i)(^|[._/-])(test|spec)([._/-]|$)", str(p.relative_to(copied_root))
        )
    ]
    ci_files = list((copied_root / ".github" / "workflows").glob("*.yml")) + list(
        (copied_root / ".github" / "workflows").glob("*.yaml")
    )
    docker_files = [p for p in files if p.name.lower().startswith("dockerfile")]
    compose_files = [
        p
        for p in files
        if p.name.lower()
        in {"docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"}
    ]

    largest = sorted(sizes, key=lambda item: item[1], reverse=True)[:15]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Project Summary ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("=" * 100 + "\n\n")
        write_key_value_lines(
            out,
            {
                "Source root": str(source_root),
                "Copied project root": str(copied_root),
                "Project name": copied_root.name,
                "Total files": f"{len(files):,}",
                "Total folders": f"{len(dirs):,}",
                "Total copied size": format_bytes(int(inventory["total_size"])),
                "README present": "yes" if readmes else "no",
                "LICENSE present": "yes" if licenses else "no",
                "Tests detected": (
                    f"yes ({len(test_files):,} files)" if test_files else "no"
                ),
                "Docker detected": "yes" if docker_files or compose_files else "no",
                "CI/CD detected": (
                    f"yes ({len(ci_files):,} GitHub Actions workflows)"
                    if ci_files
                    else "no"
                ),
                ".env-like files": f"{len(env_files):,}",
            },
        )

        out.write("\n--- Detected stack ---\n")
        for group, values in stack.items():
            out.write(f"{group}: {', '.join(values) if values else 'not detected'}\n")

        out.write("\n--- Detected languages by file count ---\n")
        language_count: Counter[str] = inventory["language_count"]
        if language_count:
            for language, count in language_count.most_common(30):
                size = inventory["language_size"][language]
                out.write(
                    f"{language:<28} {count:>8,} files   {format_bytes(size):>12}\n"
                )
        else:
            out.write("No known language extensions detected.\n")

        out.write("\n--- Largest files ---\n")
        for path, size in largest:
            out.write(f"{format_bytes(size):>12}  {rel_display(path, copied_root)}\n")

        out.write("\n--- Useful next checks ---\n")
        if not readmes:
            out.write("- Add or update README with setup/run instructions.\n")
        if not licenses:
            out.write("- Add LICENSE if this project will be shared externally.\n")
        if env_files:
            out.write("- Review .env-like files before sharing the export.\n")
        if not test_files:
            out.write(
                "- No obvious test files found; consider adding smoke/unit tests.\n"
            )
        if not ci_files:
            out.write(
                "- No GitHub Actions workflow detected; consider adding CI for checks.\n"
            )


def write_file_statistics_report(
    copied_root: Path, output_file: Path, inventory: dict[str, Any]
) -> None:
    files: list[Path] = inventory["files"]
    sizes: list[tuple[Path, int]] = inventory["sizes"]
    by_ext_count: Counter[str] = inventory["by_ext_count"]
    by_ext_size: Counter[str] = inventory["by_ext_size"]

    empty_files = [(p, s) for p, s in sizes if s == 0]
    spaced = [p for p in files if " " in p.name]
    non_ascii = [p for p in files if is_non_ascii(str(p.relative_to(copied_root)))]
    deepest = sorted(files, key=lambda p: path_depth(p, copied_root), reverse=True)[:25]
    largest = sorted(sizes, key=lambda item: item[1], reverse=True)[:25]
    long_paths = [p for p in files if len(str(p)) >= 240]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== File Statistics ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("=" * 100 + "\n\n")

        out.write("--- Files by extension ---\n")
        out.write(f"{'Extension':<20} {'Files':>10} {'Total size':>14}\n")
        out.write(f"{'-' * 20} {'-' * 10} {'-' * 14}\n")
        for ext, count in by_ext_count.most_common():
            out.write(f"{ext:<20} {count:>10,} {format_bytes(by_ext_size[ext]):>14}\n")

        out.write("\n--- Top 25 largest files ---\n")
        for path, size in largest:
            out.write(f"{format_bytes(size):>12}  {rel_display(path, copied_root)}\n")

        out.write("\n--- Top 25 deepest files ---\n")
        for path in deepest:
            out.write(
                f"depth={path_depth(path, copied_root):>2}  {rel_display(path, copied_root)}\n"
            )

        out.write("\n--- Empty files ---\n")
        if empty_files:
            for path, _size in empty_files[:100]:
                out.write(f"{rel_display(path, copied_root)}\n")
            if len(empty_files) > 100:
                out.write(f"... and {len(empty_files) - 100:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Files with spaces in names ---\n")
        if spaced:
            for path in spaced[:100]:
                out.write(f"{rel_display(path, copied_root)}\n")
            if len(spaced) > 100:
                out.write(f"... and {len(spaced) - 100:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Files with non-ASCII paths ---\n")
        if non_ascii:
            for path in non_ascii[:100]:
                out.write(f"{rel_display(path, copied_root)}\n")
            if len(non_ascii) > 100:
                out.write(f"... and {len(non_ascii) - 100:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Potential Windows long-path risks >= 240 characters ---\n")
        if long_paths:
            for path in long_paths[:100]:
                out.write(
                    f"{len(str(path)):>4} chars  {rel_display(path, copied_root)}\n"
                )
            if len(long_paths) > 100:
                out.write(f"... and {len(long_paths) - 100:,} more\n")
        else:
            out.write("None detected.\n")


def parse_go_mod(path: Path) -> tuple[str, list[str]]:
    module_name = ""
    requirements: list[str] = []
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except Exception:
        return module_name, requirements

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
                    for name, version in sorted(
                        deps.items(), key=lambda item: item[0].lower()
                    ):
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
            out.write(
                "Cargo.toml detected. Full TOML parsing is intentionally not performed without external dependencies.\n"
            )


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
            for name, command in sorted(
                scripts.items(), key=lambda item: item[0].lower()
            ):
                out.write(f"{manager} run {name:<24} # {command}\n")
        else:
            out.write("No package.json scripts detected.\n")

        out.write("\n--- Makefile targets ---\n")
        makefile = next(
            (
                p
                for p in (copied_root / "Makefile", copied_root / "makefile")
                if p.exists()
            ),
            None,
        )
        if makefile:
            try:
                targets = []
                for line in makefile.read_text(
                    encoding="utf-8", errors="replace"
                ).splitlines():
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


def write_git_deep_report(
    source_root: Path,
    output_file: Path,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> None:
    commands: list[list[str]] = [
        ["git", "status", "--short", "--branch"],
        ["git", "branch", "--show-current"],
        ["git", "branch", "-a"],
        ["git", "remote", "-v"],
        ["git", "diff", "--stat"],
        ["git", "diff", "--name-only"],
        ["git", "ls-files"],
        ["git", "ls-files", "--others", "--exclude-standard"],
        ["git", "log", "--oneline", "--decorate", "--graph", "-20"],
        ["git", "rev-parse", "--show-toplevel"],
    ]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Git Deep Report ===\n")
        out.write(f"Source root: {source_root}\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(
            "Important: this report does not switch branches and does not modify the repository.\n"
        )
        out.write("=" * 100 + "\n\n")

        if not (source_root / ".git").exists():
            out.write("No .git directory was found in the selected root.\n")
            return

        for command in commands:
            if cancel.is_set():
                out.write("\nOperation cancelled by user.\n")
                return
            command_text = " ".join(command)
            log(f"Git insight: {command_text}")
            out.write("\n" + "=" * 100 + "\n")
            out.write(f"$ {command_text}\n")
            out.write("=" * 100 + "\n\n")
            code, stdout, stderr = run_git_command(
                command, cwd=source_root, timeout_seconds=120
            )
            out.write(f"Exit code: {code}\n\n")
            out.write("--- STDOUT ---\n")
            out.write(stdout or "")
            if stdout and not stdout.endswith("\n"):
                out.write("\n")
            out.write("\n--- STDERR ---\n")
            out.write(stderr or "")
            if stderr and not stderr.endswith("\n"):
                out.write("\n")
            out.write("\n")


def redacted_line(line: str) -> str:
    line = redact_secrets(line)
    if SECRET_KEY_PATTERN.search(line):
        if "=" in line:
            key = line.split("=", 1)[0].strip()
            return f"{key}=<REDACTED>"
        if ":" in line:
            key = line.split(":", 1)[0].strip()
            return f"{key}: <REDACTED>"
        return "<REDACTED_SECRET_LINE>"
    return line.strip()


def write_security_scan_report(
    copied_root: Path, output_file: Path, max_bytes_per_file: int
) -> None:
    suspicious_files: list[Path] = []
    suspicious_lines: list[tuple[Path, int, str]] = []

    for path in iter_project_files(copied_root):
        name = path.name.lower()
        suffix = path.suffix.lower().lstrip(".")
        if (
            name in SENSITIVE_FILENAMES
            or suffix in SENSITIVE_SUFFIXES
            or name.startswith(".env")
        ):
            suspicious_files.append(path)

        if not should_consider_text_file(path):
            continue
        try:
            if path.stat().st_size > max_bytes_per_file:
                continue
        except Exception:
            continue

        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue
        for line_number, line in enumerate(text.splitlines(), start=1):
            if SECRET_KEY_PATTERN.search(line) or any(
                pattern.search(line) for pattern in SECRET_PATTERNS
            ):
                suspicious_lines.append((path, line_number, redacted_line(line)))

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Basic Security Scan ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write(
            "This is a heuristic scan, not a professional secret scanner. Values are redacted.\n"
        )
        out.write("=" * 100 + "\n\n")

        out.write("--- Sensitive-looking files ---\n")
        if suspicious_files:
            for path in sorted(suspicious_files, key=lambda p: str(p).lower())[:300]:
                out.write(f"{rel_display(path, copied_root)}\n")
            if len(suspicious_files) > 300:
                out.write(f"... and {len(suspicious_files) - 300:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Potential secret-like lines ---\n")
        if suspicious_lines:
            for path, line_number, line in suspicious_lines[:500]:
                out.write(f"{rel_display(path, copied_root)}:{line_number}: {line}\n")
            if len(suspicious_lines) > 500:
                out.write(f"... and {len(suspicious_lines) - 500:,} more\n")
        else:
            out.write("None detected.\n")

        out.write("\n--- Recommended actions before sharing ---\n")
        out.write("- Review all .env-like files.\n")
        out.write(
            "- Rotate any secret that may have been committed or exported by mistake.\n"
        )
        out.write("- Prefer .env.example with placeholder values for shared exports.\n")


def write_todo_fixme_report(
    copied_root: Path, output_file: Path, max_bytes_per_file: int
) -> None:
    findings: list[tuple[Path, int, str, str]] = []
    for path in iter_project_files(copied_root):
        if not should_consider_text_file(path):
            continue
        try:
            if path.stat().st_size > max_bytes_per_file:
                continue
        except Exception:
            continue
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue
        for line_number, line in enumerate(text.splitlines(), start=1):
            match = TODO_PATTERN.search(line)
            if match:
                findings.append(
                    (path, line_number, match.group(1).upper(), line.strip())
                )

    by_kind: Counter[str] = Counter(kind for _path, _line, kind, _text in findings)

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== TODO / FIXME / Technical Debt Report ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("=" * 100 + "\n\n")

        out.write("--- Summary ---\n")
        if by_kind:
            for kind, count in by_kind.most_common():
                out.write(f"{kind:<14} {count:>8,}\n")
        else:
            out.write("No TODO/FIXME-like markers detected.\n")

        out.write("\n--- Findings ---\n")
        if findings:
            for path, line_number, kind, line in findings[:1000]:
                out.write(
                    f"{rel_display(path, copied_root)}:{line_number}: [{kind}] {line}\n"
                )
            if len(findings) > 1000:
                out.write(f"... and {len(findings) - 1000:,} more\n")
        else:
            out.write("None.\n")


def comment_like_line(stripped: str, suffix: str) -> bool:
    if not stripped:
        return False
    common = ("//", "/*", "*", "#", "<!--", "--")
    if stripped.startswith(common):
        return True
    if suffix in {"sql"} and stripped.startswith("--"):
        return True
    return False


def write_code_metrics_report(
    copied_root: Path, output_file: Path, max_bytes_per_file: int
) -> None:
    per_file: list[dict[str, Any]] = []
    totals = Counter()

    for path in iter_project_files(copied_root):
        suffix = extension_key(path)
        if suffix not in SOURCE_CODE_EXTENSIONS:
            continue
        try:
            if path.stat().st_size > max_bytes_per_file:
                continue
        except Exception:
            continue
        text, _info = read_text_safely(path, max_bytes=max_bytes_per_file)
        if text is None:
            continue

        lines = text.splitlines()
        blank = 0
        comments = 0
        for line in lines:
            stripped = line.strip()
            if not stripped:
                blank += 1
            elif comment_like_line(stripped, suffix):
                comments += 1
        code = max(0, len(lines) - blank - comments)
        item = {
            "path": path,
            "lines": len(lines),
            "blank": blank,
            "comments": comments,
            "code": code,
            "suffix": suffix,
        }
        per_file.append(item)
        totals["lines"] += len(lines)
        totals["blank"] += blank
        totals["comments"] += comments
        totals["code"] += code

    largest_by_lines = sorted(per_file, key=lambda item: item["lines"], reverse=True)[
        :50
    ]
    over_500 = [item for item in per_file if item["lines"] >= 500]
    over_1000 = [item for item in per_file if item["lines"] >= 1000]
    by_ext: dict[str, Counter[str]] = defaultdict(Counter)
    for item in per_file:
        ext = item["suffix"]
        by_ext[ext]["files"] += 1
        by_ext[ext]["lines"] += item["lines"]
        by_ext[ext]["code"] += item["code"]

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Code Metrics ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("Line classification is heuristic.\n")
        out.write("=" * 100 + "\n\n")
        write_key_value_lines(
            out,
            {
                "Source files analysed": f"{len(per_file):,}",
                "Total lines": f"{totals['lines']:,}",
                "Code lines": f"{totals['code']:,}",
                "Comment-like lines": f"{totals['comments']:,}",
                "Blank lines": f"{totals['blank']:,}",
                "Files >= 500 lines": f"{len(over_500):,}",
                "Files >= 1000 lines": f"{len(over_1000):,}",
            },
        )

        out.write("\n--- Metrics by extension ---\n")
        out.write(f"{'Ext':<16} {'Files':>8} {'Lines':>12} {'Code':>12}\n")
        for ext, counter in sorted(
            by_ext.items(), key=lambda item: item[1]["lines"], reverse=True
        ):
            out.write(
                f"{ext:<16} {counter['files']:>8,} {counter['lines']:>12,} {counter['code']:>12,}\n"
            )

        out.write("\n--- Largest files by line count ---\n")
        for item in largest_by_lines:
            out.write(
                f"{item['lines']:>8,} lines  code={item['code']:>8,}  {rel_display(item['path'], copied_root)}\n"
            )

        out.write("\n--- Files >= 500 lines ---\n")
        if over_500:
            for item in sorted(over_500, key=lambda x: x["lines"], reverse=True):
                out.write(
                    f"{item['lines']:>8,} lines  {rel_display(item['path'], copied_root)}\n"
                )
        else:
            out.write("None detected.\n")


def find_config_files(copied_root: Path) -> list[Path]:
    known: list[Path] = []
    for path in iter_project_files(copied_root):
        rel = str(path.relative_to(copied_root)).replace("\\", "/")
        name = path.name
        lower_name = name.lower()
        if name in CONFIG_FILES or lower_name in {
            item.lower() for item in CONFIG_FILES
        }:
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
            "eslint" in p.name.lower() or p.name.lower().startswith(".eslintrc")
            for p in configs
        ),
        "Prettier": any(
            "prettier" in p.name.lower() or p.name.lower().startswith(".prettierrc")
            for p in configs
        ),
        "Tailwind CSS": any(
            p.name.lower().startswith("tailwind.config") for p in configs
        ),
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
            str(p.relative_to(copied_root))
            .replace("\\", "/")
            .startswith(".github/workflows/")
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
            services[current_service].setdefault(current_key, []).append(
                stripped[1:].strip()
            )
    return services


def write_docker_report(copied_root: Path, output_file: Path) -> None:
    dockerfiles = sorted(
        [
            p
            for p in iter_project_files(copied_root)
            if p.name.lower().startswith("dockerfile")
        ],
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
        out.write(
            "Compose parsing is heuristic and works best with simple YAML files.\n"
        )
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
            out.write(
                f"\n--- Parsed services from {rel_display(compose, copied_root)} ---\n"
            )
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


def write_routes_and_pages_report(copied_root: Path, output_file: Path) -> None:
    interesting_dirs = {
        "pages": [],
        "routes": [],
        "app": [],
        "features": [],
        "components": [],
        "widgets": [],
        "layouts": [],
    }

    for directory in iter_project_dirs(copied_root):
        lower_parts = [
            part.lower() for part in directory.relative_to(copied_root).parts
        ]
        for key in interesting_dirs:
            if key in lower_parts:
                interesting_dirs[key].append(directory)
                break

    route_like_files: list[Path] = []
    component_like_files: list[Path] = []
    page_like_files: list[Path] = []

    for path in iter_project_files(copied_root):
        rel_parts = [part.lower() for part in path.relative_to(copied_root).parts]
        suffix = extension_key(path)
        if suffix not in {"ts", "tsx", "js", "jsx", "vue", "svelte", "astro"}:
            continue
        if (
            "routes" in rel_parts
            or "router" in path.name.lower()
            or "route" in path.name.lower()
        ):
            route_like_files.append(path)
        if "components" in rel_parts or re.match(
            r"^[A-Z].*\.(tsx|jsx|vue|svelte|astro)$", path.name
        ):
            component_like_files.append(path)
        if "pages" in rel_parts or "page" in path.name.lower():
            page_like_files.append(path)

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write("=== Frontend Routes / Pages / UI Map ===\n")
        out.write(f"Generated: {human_now()}\n")
        out.write("This is a heuristic map based on folder and file names.\n")
        out.write("=" * 100 + "\n\n")

        out.write("--- Important UI directories ---\n")
        for key, dirs in interesting_dirs.items():
            out.write(f"\n{key}:\n")
            if dirs:
                for directory in sorted(set(dirs), key=lambda p: str(p).lower())[:100]:
                    out.write(f"- {rel_display(directory, copied_root)}\n")
            else:
                out.write("- not detected\n")

        groups = (
            ("Route-like files", route_like_files),
            ("Page-like files", page_like_files),
            ("Component-like files", component_like_files),
        )
        for title, paths in groups:
            out.write(f"\n--- {title} ---\n")
            if paths:
                for path in sorted(set(paths), key=lambda p: str(p).lower())[:300]:
                    out.write(f"{rel_display(path, copied_root)}\n")
                if len(paths) > 300:
                    out.write(f"... and {len(paths) - 300:,} more\n")
            else:
                out.write("None detected.\n")


def write_ai_context_pack(
    copied_root: Path, source_root: Path, output_file: Path, inventory: dict[str, Any]
) -> None:
    files: list[Path] = inventory["files"]
    dirs: list[Path] = inventory["dirs"]
    stack: dict[str, list[str]] = inventory["stack"]
    largest = sorted(inventory["sizes"], key=lambda item: item[1], reverse=True)[:10]
    language_count: Counter[str] = inventory["language_count"]
    configs = find_config_files(copied_root)
    package_json = safe_read_json(copied_root / "package.json")
    scripts = package_json.get("scripts") if package_json else None

    with output_file.open("w", encoding="utf-8", newline="\n", errors="replace") as out:
        out.write(f"# AI Context Pack: {copied_root.name}\n\n")
        out.write(f"Generated: {human_now()}\n\n")
        out.write(
            "This file is intended to be pasted into ChatGPT/Codex together with the exported project when quick project understanding is needed.\n\n"
        )

        out.write("## Project summary\n\n")
        out.write(f"- Source root: `{source_root}`\n")
        out.write(f"- Copied root: `{copied_root}`\n")
        out.write(f"- Files: {len(files):,}\n")
        out.write(f"- Folders: {len(dirs):,}\n")
        out.write(f"- Copied size: {format_bytes(int(inventory['total_size']))}\n")

        out.write("\n## Detected stack\n\n")
        for group, values in stack.items():
            out.write(
                f"- **{group}**: {', '.join(values) if values else 'not detected'}\n"
            )

        out.write("\n## Main languages\n\n")
        if language_count:
            for language, count in language_count.most_common(15):
                out.write(f"- {language}: {count:,} files\n")
        else:
            out.write("- No known language extensions detected.\n")

        out.write("\n## Scripts / commands\n\n")
        if isinstance(scripts, dict) and scripts:
            manager = "pnpm" if (copied_root / "pnpm-lock.yaml").exists() else "npm"
            for name, command in sorted(
                scripts.items(), key=lambda item: item[0].lower()
            ):
                out.write(f"- `{manager} run {name}` — `{command}`\n")
        else:
            out.write("- No package.json scripts detected.\n")

        out.write("\n## Important configuration files\n\n")
        if configs:
            for path in configs[:80]:
                out.write(f"- `{rel_display(path, copied_root)}`\n")
        else:
            out.write("- No common configuration files detected.\n")

        out.write("\n## Largest files\n\n")
        for path, size in largest:
            out.write(f"- `{rel_display(path, copied_root)}` — {format_bytes(size)}\n")

        out.write("\n## Suggested review order\n\n")
        suggestions = [
            "Read `01_summary.txt` first.",
            "Review `05_git_deep.txt` for branch, remote, and uncommitted state.",
            "Review `06_security_scan.txt` before sharing the export.",
            "Review `07_todo_fixme.txt` to understand technical debt.",
            "Use `08_code_metrics.txt` to find files that may need decomposition.",
        ]
        for suggestion in suggestions:
            out.write(f"- {suggestion}\n")


def write_project_insight_reports(
    copied_root: Path,
    source_root: Path,
    reports_dir: Path,
    max_bytes_per_file: int,
    log: Callable[[str], None],
    cancel: threading.Event,
) -> None:
    reports_dir.mkdir(parents=True, exist_ok=True)
    log(f"Создаю расширенные отчёты: {reports_dir}")

    inventory = collect_basic_inventory(copied_root)

    report_jobs = [
        (
            "01_summary.txt",
            lambda output: write_project_summary_report(
                copied_root, source_root, output, inventory
            ),
        ),
        (
            "02_file_statistics.txt",
            lambda output: write_file_statistics_report(copied_root, output, inventory),
        ),
        (
            "03_dependencies.txt",
            lambda output: write_dependency_report(copied_root, output),
        ),
        (
            "04_scripts.txt",
            lambda output: write_scripts_report(copied_root, output),
        ),
        (
            "05_git_deep.txt",
            lambda output: write_git_deep_report(source_root, output, log, cancel),
        ),
        (
            "06_security_scan.txt",
            lambda output: write_security_scan_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "07_todo_fixme.txt",
            lambda output: write_todo_fixme_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "08_code_metrics.txt",
            lambda output: write_code_metrics_report(
                copied_root, output, max_bytes_per_file
            ),
        ),
        (
            "09_config.txt",
            lambda output: write_config_report(copied_root, output),
        ),
        (
            "10_docker.txt",
            lambda output: write_docker_report(copied_root, output),
        ),
        (
            "11_routes_and_pages.txt",
            lambda output: write_routes_and_pages_report(copied_root, output),
        ),
        (
            "12_ai_context_pack.md",
            lambda output: write_ai_context_pack(
                copied_root, source_root, output, inventory
            ),
        ),
    ]

    for filename, writer in report_jobs:
        if cancel.is_set():
            log("Создание расширенных отчётов остановлено пользователем.")
            break
        output_file = reports_dir / filename
        log(f"Пишу отчёт: {filename}")
        try:
            writer(output_file)
        except Exception as exc:
            error_file = reports_dir / f"ERROR_{filename}.txt"
            error_file.write_text(
                f"Failed to create {filename}\n\n{type(exc).__name__}: {exc}\n\n{traceback.format_exc()}",
                encoding="utf-8",
                errors="replace",
            )
            log(f"Ошибка создания {filename}: {exc}")

    log("Расширенные аналитические отчёты готовы")


# ---------------------------------------------------------------------------
# Bundle metadata: manifest.json + INDEX.md
# ---------------------------------------------------------------------------

# Human-readable descriptions of every report file. Single source of truth
# for both the INDEX.md and manifest.json so they never drift apart.
REPORT_DESCRIPTIONS: tuple[tuple[str, str], ...] = (
    (
        "reports/01_structure.txt",
        "PowerShell-like directory listing of the copied project.",
    ),
    (
        "reports/02_git.txt",
        "Read-only Git snapshot from the ORIGINAL project (status, last commits).",
    ),
    (
        "reports/03_text_dump.txt",
        "Concatenated contents of every text-like file in the copy.",
    ),
    (
        "reports/insights/01_summary.txt",
        "High-level overview: stack detection, language counts, biggest files.",
    ),
    (
        "reports/insights/02_file_statistics.txt",
        "File counts by extension, deepest paths, empty / suspicious files.",
    ),
    (
        "reports/insights/03_dependencies.txt",
        "package.json / requirements.txt / go.mod / Cargo.toml dependencies.",
    ),
    (
        "reports/insights/04_scripts.txt",
        "npm/pnpm scripts, Makefile targets, Docker convenience commands.",
    ),
    (
        "reports/insights/05_git_deep.txt",
        "Extended read-only Git inspection (branches, remotes, diffs, ls-files).",
    ),
    (
        "reports/insights/06_security_scan.txt",
        "Heuristic scan for .env-like files and secret-looking lines (redacted).",
    ),
    (
        "reports/insights/07_todo_fixme.txt",
        "TODO / FIXME / HACK / XXX / DEPRECATED markers across the codebase.",
    ),
    (
        "reports/insights/08_code_metrics.txt",
        "LOC, comment ratios, files over 500 / 1000 lines.",
    ),
    (
        "reports/insights/09_config.txt",
        "Detected configuration files and capability checklist.",
    ),
    ("reports/insights/10_docker.txt", "Dockerfile / docker-compose service map."),
    (
        "reports/insights/11_routes_and_pages.txt",
        "Heuristic UI map: routes, pages, components.",
    ),
    (
        "reports/insights/12_ai_context_pack.md",
        "Drop-in summary for pasting into an LLM along with the project.",
    ),
)


def write_manifest(
    paths: ExportPaths,
    config: Config,
    copy_stats: CopyStats,
    text_stats: TextDumpStats,
    extra_ignored_dirs: frozenset[str] | set[str],
    cancelled: bool,
) -> None:
    """Write a machine-readable description of the bundle."""
    data = {
        "app": APP_NAME,
        "app_version": APP_VERSION,
        "generated_at": human_now(),
        "bundle_name": paths.bundle_name,
        "source_root": str(paths.source_root),
        "project_name": paths.project_name,
        "cancelled": cancelled,
        "settings": {
            "max_text_file_mb": config.max_text_file_mb,
            "redact_secrets": config.redact_secrets,
            "keep_staging_folder": config.keep_staging_folder,
            "include_project_in_zip": config.include_project_in_zip,
        },
        "ignored_dirs": {
            "defaults": sorted(IGNORED_DIR_NAMES),
            "user_extras": sorted(set(extra_ignored_dirs) - IGNORED_DIR_NAMES),
            "effective": sorted(IGNORED_DIR_NAMES | set(extra_ignored_dirs)),
        },
        "notes": [
            "Git data is collected from the ORIGINAL project; the .git "
            "directory is never copied into the bundle.",
            "All other reports describe the copied project (see "
            f"'{paths.project_name}/' inside the bundle).",
            "Symlinks were skipped during copy to avoid accidental escape "
            "from the project tree.",
        ],
        "stats": {
            "copy": asdict(copy_stats),
            "text_dump": asdict(text_stats),
        },
        "layout": {
            "project_dir": paths.project_name + "/",
            "reports": [path for path, _desc in REPORT_DESCRIPTIONS],
        },
    }

    paths.manifest_file.write_text(
        json.dumps(data, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def write_index_md(
    paths: ExportPaths,
    config: Config,
    extra_ignored_dirs: frozenset[str] | set[str],
) -> None:
    """Write a human-readable bundle table of contents."""
    ignored_effective = sorted(IGNORED_DIR_NAMES | set(extra_ignored_dirs))

    with paths.index_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write(f"# Export bundle: `{paths.project_name}`\n\n")
        out.write(f"_Generated by {APP_NAME} v{APP_VERSION} on {human_now()}._\n\n")
        out.write(
            "This archive bundles a copy of a project together with a set of "
            "machine- and human-readable reports describing it from several "
            "angles.\n\n"
        )

        out.write("## Bundle contents\n\n")
        out.write(
            f"- `{paths.project_name}/` — working copy of the project "
            f"(without `.git`, `node_modules`, and any user-configured extras).\n"
        )
        out.write("- `manifest.json` — machine-readable metadata about this export.\n")
        out.write("- `reports/` — generated reports (see below).\n\n")

        out.write("## Reports\n\n")
        for rel_path, description in REPORT_DESCRIPTIONS:
            out.write(f"- `{rel_path}` — {description}\n")

        out.write("\n## Settings used for this export\n\n")
        out.write(f"- Max text-file size: **{config.max_text_file_mb} MB**\n")
        out.write(
            f"- Secret redaction: "
            f"**{'enabled' if config.redact_secrets else 'disabled'}**\n"
        )
        out.write(
            f"- Project included in ZIP: "
            f"**{'yes' if config.include_project_in_zip else 'no (reports only)'}**\n"
        )
        out.write(
            f"- Staging folder kept on Desktop: "
            f"**{'yes' if config.keep_staging_folder else 'no'}**\n"
        )
        out.write(
            f"- Ignored directories: "
            + ", ".join(f"`{n}`" for n in ignored_effective)
            + "\n"
        )

        out.write("\n## Notes\n\n")
        out.write(
            "- Git data was collected from the **original** project. The "
            "`.git` directory itself is intentionally not part of the bundle.\n"
        )
        out.write(
            "- All other reports describe the **copy** of the project that "
            f"lives in `{paths.project_name}/` inside this bundle.\n"
        )
        out.write(
            "- Symbolic links were skipped during the copy to avoid accidental "
            "escape from the project tree.\n"
        )


# ---------------------------------------------------------------------------
# Orchestrator
# ---------------------------------------------------------------------------


class ProjectExporter:
    def __init__(
        self,
        source_root: Path,
        config: Config,
        log_queue: Queue[str],
        cancel_event: threading.Event,
    ):
        self.source_root = source_root
        self.config = config
        self.log_queue = log_queue
        self.cancel_event = cancel_event

    def log(self, message: str) -> None:
        self.log_queue.put(message)

    def run(self) -> ExportPaths:
        paths = build_export_paths(self.source_root)
        extra_ignored = self.config.effective_ignored_dirs() - IGNORED_DIR_NAMES
        ignored_for_walk = self.config.effective_ignored_dirs()
        max_bytes = max(1, self.config.max_text_file_mb) * 1024 * 1024
        cancelled = False

        self.log(f"Итоговая папка-stage: {paths.staging_dir}")
        self.log(f"Итоговый ZIP: {paths.final_zip}")
        if extra_ignored:
            self.log(
                "Дополнительные исключаемые папки: " + ", ".join(sorted(extra_ignored))
            )

        # --- Step 1/7: copy ---------------------------------------------------
        self.log("Шаг 1/7: копирование проекта")
        paths.staging_dir.mkdir(parents=True, exist_ok=True)
        paths.reports_dir.mkdir(parents=True, exist_ok=True)
        paths.insights_dir.mkdir(parents=True, exist_ok=True)

        copy_stats = copy_project(
            source_root=paths.source_root,
            destination_root=paths.project_dir,
            extra_ignored_dirs=ignored_for_walk,
            log=self.log,
            cancel=self.cancel_event,
        )

        self.log(
            "Копирование завершено: "
            f"files={copy_stats.files_copied:,}, dirs={copy_stats.dirs_created:,}, "
            f"skipped_dirs={copy_stats.dirs_skipped:,}, "
            f"symlinks_skipped={copy_stats.symlinks_skipped:,}, "
            f"errors={copy_stats.errors:,}"
        )

        text_stats = TextDumpStats()

        if self.cancel_event.is_set():
            cancelled = True
        else:
            # --- Step 2/7: directory structure --------------------------------
            self.log("Шаг 2/7: запись относительной структуры")
            write_structure_report(
                paths.project_dir,
                paths.structure_report,
                ignored_for_walk,
                self.log,
                self.cancel_event,
            )

        if not cancelled and not self.cancel_event.is_set():
            # --- Step 3/7: basic Git report -----------------------------------
            self.log("Шаг 3/7: выполнение Git-команд")
            write_git_report(
                paths.source_root, paths.git_report, self.log, self.cancel_event
            )

        if not cancelled and not self.cancel_event.is_set():
            # --- Step 4/7: text dump ------------------------------------------
            self.log("Шаг 4/7: сбор текстового содержимого")
            text_stats = write_text_dump(
                root=paths.project_dir,
                output_file=paths.text_dump,
                max_bytes_per_file=max_bytes,
                redact=self.config.redact_secrets,
                log=self.log,
                cancel=self.cancel_event,
            )
            self.log(
                "Текстовый отчёт: "
                f"scanned={text_stats.scanned:,}, written={text_stats.written:,}, "
                f"binary={text_stats.skipped_binary:,}, "
                f"large={text_stats.skipped_large:,}, "
                f"not_text={text_stats.skipped_not_text:,}, "
                f"decode_errors={text_stats.skipped_decode:,}"
            )

        if not cancelled and not self.cancel_event.is_set():
            # --- Step 5/7: project insights -----------------------------------
            self.log("Шаг 5/7: расширенная аналитика проекта")
            write_project_insight_reports(
                copied_root=paths.project_dir,
                source_root=paths.source_root,
                reports_dir=paths.insights_dir,
                max_bytes_per_file=max_bytes,
                log=self.log,
                cancel=self.cancel_event,
            )

        # --- Step 6/7: manifest + INDEX (always written) ----------------------
        # Even if cancelled, we still record what happened.
        cancelled = cancelled or self.cancel_event.is_set()
        self.log("Шаг 6/7: запись manifest.json и INDEX.md")
        write_manifest(
            paths=paths,
            config=self.config,
            copy_stats=copy_stats,
            text_stats=text_stats,
            extra_ignored_dirs=ignored_for_walk,
            cancelled=cancelled,
        )
        write_index_md(
            paths=paths,
            config=self.config,
            extra_ignored_dirs=ignored_for_walk,
        )

        # --- Step 7/7: final zip + optional cleanup ---------------------------
        self.log("Шаг 7/7: упаковка итогового ZIP")
        build_final_zip(
            paths=paths,
            include_project=self.config.include_project_in_zip,
            log=self.log,
            cancel=self.cancel_event,
        )

        if not self.config.keep_staging_folder:
            self.log("Удаляю промежуточную папку (staging)")
            try:
                shutil.rmtree(paths.staging_dir, ignore_errors=False)
            except Exception as exc:
                self.log(f"Не удалось удалить staging-папку: {exc}")

        if cancelled:
            self.log(
                "Готово (с прерыванием). Итоговый ZIP содержит то, что успело собраться."
            )
        else:
            self.log("Готово. Итоговый ZIP лежит на Desktop.")

        return paths


# ---------------------------------------------------------------------------
# Tkinter GUI
# ---------------------------------------------------------------------------


class App:
    def __init__(self, master: tk.Tk):
        self.master = master
        self.config = Config.load()
        self.log_queue: Queue[str] = Queue()
        self.cancel_event = threading.Event()
        self.worker: threading.Thread | None = None
        self.last_result_path: Path | None = None  # zip or staging dir

        self._build_ui()
        self._load_config_to_ui()
        self._poll_logs()

    # -- UI construction ----------------------------------------------------

    def _build_ui(self) -> None:
        self.master.title(f"{APP_NAME} v{APP_VERSION}")
        self.master.geometry("960x760")
        self.master.minsize(880, 640)

        root = ttk.Frame(self.master, padding=14)
        root.pack(fill="both", expand=True)

        title = ttk.Label(root, text=APP_NAME, font=("Segoe UI", 16, "bold"))
        title.grid(row=0, column=0, columnspan=3, sticky="w")

        subtitle = ttk.Label(
            root,
            text=(
                "Один ZIP с копией проекта, manifest.json, INDEX.md "
                "и пакетом отчётов (структура, Git, текстовый дамп, аналитика)."
            ),
            foreground="gray",
        )
        subtitle.grid(row=1, column=0, columnspan=3, sticky="w", pady=(2, 14))

        ttk.Label(root, text="Корневая папка проекта:").grid(
            row=2, column=0, columnspan=3, sticky="w"
        )

        self.entry_root = ttk.Entry(root)
        self.entry_root.grid(row=3, column=0, sticky="we", padx=(0, 8))

        ttk.Button(root, text="Обзор", command=self._browse_root).grid(
            row=3, column=1, sticky="e"
        )
        ttk.Button(root, text="Открыть Desktop", command=self._open_desktop).grid(
            row=3, column=2, sticky="e", padx=(8, 0)
        )

        options = ttk.LabelFrame(root, text="Настройки", padding=10)
        options.grid(row=4, column=0, columnspan=3, sticky="we", pady=(14, 10))

        # Row: max text file size
        size_line = ttk.Frame(options)
        size_line.pack(anchor="w", fill="x")
        ttk.Label(size_line, text="Максимальный размер одного текстового файла:").pack(
            side="left"
        )
        self.var_max_mb = tk.StringVar()
        self.entry_max_mb = ttk.Entry(
            size_line, width=8, textvariable=self.var_max_mb, justify="right"
        )
        self.entry_max_mb.pack(side="left", padx=(8, 4))
        ttk.Label(size_line, text="МБ").pack(side="left")

        # Row: redact secrets
        self.var_redact = tk.BooleanVar(value=True)
        ttk.Checkbutton(
            options,
            text=(
                "Маскировать очевидные секреты в текстовом дампе "
                "(.env, TOKEN, PASSWORD, API_KEY и т.п.)"
            ),
            variable=self.var_redact,
        ).pack(anchor="w", pady=(8, 0))

        # Row: include project in zip
        self.var_include_project = tk.BooleanVar(value=True)
        ttk.Checkbutton(
            options,
            text="Включать копию проекта внутрь итогового ZIP (иначе — только отчёты)",
            variable=self.var_include_project,
        ).pack(anchor="w", pady=(4, 0))

        # Row: keep staging
        self.var_keep_staging = tk.BooleanVar(value=False)
        ttk.Checkbutton(
            options,
            text=(
                "Оставить распакованную папку рядом с ZIP "
                "(удобно для просмотра без распаковки)"
            ),
            variable=self.var_keep_staging,
        ).pack(anchor="w", pady=(4, 0))

        # Row: extra ignored dirs
        extras_line = ttk.Frame(options)
        extras_line.pack(anchor="w", fill="x", pady=(8, 0))
        ttk.Label(
            extras_line,
            text="Дополнительно исключить папки (через запятую):",
        ).pack(side="left")
        self.var_extra_ignored = tk.StringVar()
        self.entry_extra_ignored = ttk.Entry(
            extras_line, textvariable=self.var_extra_ignored
        )
        self.entry_extra_ignored.pack(side="left", fill="x", expand=True, padx=(8, 0))

        ttk.Label(
            options,
            text=(
                "Базовое исключение .git и node_modules сохраняется всегда — "
                "ваши значения только добавляются."
            ),
            foreground="gray",
        ).pack(anchor="w", pady=(4, 0))

        warning = ttk.Label(
            options,
            text=(
                "Важно: Git-команды read-only — не переключают ветку "
                "и не изменяют исходный проект."
            ),
            foreground="gray",
        )
        warning.pack(anchor="w", pady=(8, 0))

        # Action buttons
        actions = ttk.Frame(root)
        actions.grid(row=5, column=0, columnspan=3, sticky="we", pady=(8, 10))

        self.btn_start = ttk.Button(
            actions, text="▶ Создать экспорт", command=self._start
        )
        self.btn_start.pack(side="left")

        self.btn_cancel = ttk.Button(
            actions, text="Отмена", command=self._cancel, state="disabled"
        )
        self.btn_cancel.pack(side="left", padx=8)

        self.btn_open_result = ttk.Button(
            actions,
            text="Открыть результат",
            command=self._open_last_result,
            state="disabled",
        )
        self.btn_open_result.pack(side="left")

        ttk.Button(actions, text="Сброс настроек", command=self._reset_settings).pack(
            side="right"
        )

        # Progress + status
        progress_line = ttk.Frame(root)
        progress_line.grid(row=6, column=0, columnspan=3, sticky="we", pady=(4, 8))

        self.progress = ttk.Progressbar(progress_line, mode="indeterminate")
        self.progress.pack(side="left", fill="x", expand=True)

        self.lbl_status = ttk.Label(progress_line, text="Готов", width=18)
        self.lbl_status.pack(side="left", padx=(10, 0))

        # Log
        self.log = scrolledtext.ScrolledText(
            root, height=22, state="disabled", wrap="word", font=("Consolas", 9)
        )
        self.log.grid(row=7, column=0, columnspan=3, sticky="nsew")

        footer = ttk.Label(
            root,
            text=(
                "Результат создаётся на Desktop. По умолчанию это один файл "
                "вида {project}_export_{timestamp}.zip."
            ),
            foreground="gray",
        )
        footer.grid(row=8, column=0, columnspan=3, sticky="w", pady=(8, 0))

        root.columnconfigure(0, weight=1)
        root.rowconfigure(7, weight=1)

    # -- Config sync --------------------------------------------------------

    def _load_config_to_ui(self) -> None:
        self.entry_root.delete(0, "end")
        self.entry_root.insert(0, self.config.last_root)

        self.var_max_mb.set(str(self.config.max_text_file_mb))
        self.var_redact.set(self.config.redact_secrets)
        self.var_include_project.set(self.config.include_project_in_zip)
        self.var_keep_staging.set(self.config.keep_staging_folder)
        self.var_extra_ignored.set(", ".join(self.config.extra_ignored_dirs))

    def _save_config_from_ui(self) -> None:
        self.config.last_root = self.entry_root.get().strip() or str(Path.home())
        try:
            self.config.max_text_file_mb = max(1, int(self.var_max_mb.get().strip()))
        except Exception:
            self.config.max_text_file_mb = 5
            self.var_max_mb.set("5")

        self.config.redact_secrets = bool(self.var_redact.get())
        self.config.include_project_in_zip = bool(self.var_include_project.get())
        self.config.keep_staging_folder = bool(self.var_keep_staging.get())

        raw_extras = self.var_extra_ignored.get()
        extras: list[str] = []
        for token in re.split(r"[,;\n]", raw_extras):
            token = token.strip()
            if token and token not in extras:
                extras.append(token)
        self.config.extra_ignored_dirs = extras

        self.config.save()

    # -- Buttons ------------------------------------------------------------

    def _browse_root(self) -> None:
        initial = self.entry_root.get().strip() or str(Path.home())
        selected = filedialog.askdirectory(
            initialdir=initial, title="Выберите корневую папку проекта"
        )
        if selected:
            self.entry_root.delete(0, "end")
            self.entry_root.insert(0, selected)

    def _start(self) -> None:
        if self.worker and self.worker.is_alive():
            return

        try:
            source_root = validate_source_root(self.entry_root.get())
        except Exception as exc:
            messagebox.showerror("Ошибка", str(exc))
            return

        try:
            max(1, int(self.var_max_mb.get().strip()))
        except Exception:
            messagebox.showerror(
                "Ошибка", "Максимальный размер файла должен быть целым числом."
            )
            return

        self._save_config_from_ui()
        self.cancel_event.clear()
        self.last_result_path = None
        self.btn_open_result.config(state="disabled")
        self._set_running(True)
        self._log("Запуск экспорта...")

        exporter = ProjectExporter(
            source_root=source_root,
            config=self.config,
            log_queue=self.log_queue,
            cancel_event=self.cancel_event,
        )

        def target() -> None:
            try:
                paths = exporter.run()
                # Prefer the zip as the "result"; fall back to staging if kept.
                if paths.final_zip.exists():
                    self.last_result_path = paths.final_zip
                elif paths.staging_dir.exists():
                    self.last_result_path = paths.staging_dir

                if self.cancel_event.is_set():
                    location = self.last_result_path or paths.staging_dir
                    self.master.after(
                        0,
                        lambda: messagebox.showwarning(
                            "Остановлено",
                            f"Операция остановлена.\n{location}",
                        ),
                    )
                else:
                    self.master.after(
                        0, lambda: self.btn_open_result.config(state="normal")
                    )
                    location = self.last_result_path or paths.final_zip
                    self.master.after(
                        0,
                        lambda: messagebox.showinfo(
                            "Готово", f"Экспорт создан:\n{location}"
                        ),
                    )
            except Exception:
                error_text = traceback.format_exc()
                self.log_queue.put(error_text)
                self.master.after(0, lambda: messagebox.showerror("Ошибка", error_text))
            finally:
                self.master.after(0, lambda: self._set_running(False))

        self.worker = threading.Thread(target=target, daemon=True)
        self.worker.start()

    def _cancel(self) -> None:
        if self.worker and self.worker.is_alive():
            if messagebox.askyesno("Отмена", "Остановить текущую операцию?"):
                self.cancel_event.set()
                self._log("Запрошена остановка...")

    def _set_running(self, running: bool) -> None:
        if running:
            self.btn_start.config(state="disabled")
            self.btn_cancel.config(state="normal")
            self.lbl_status.config(text="Выполняется...")
            self.progress.start(15)
        else:
            self.btn_start.config(state="normal")
            self.btn_cancel.config(state="disabled")
            self.lbl_status.config(text="Готов")
            self.progress.stop()

    def _open_path(self, path: Path) -> None:
        try:
            if os.name == "nt":
                # On Windows, startfile opens files in their default app
                # and folders in Explorer.
                os.startfile(str(path))  # type: ignore[attr-defined]
            elif sys.platform == "darwin":
                # For files: open them; for folders: reveal in Finder.
                # `open` handles both.
                subprocess.Popen(["open", str(path)])
            else:
                # Linux: xdg-open handles both files and directories.
                subprocess.Popen(["xdg-open", str(path)])
        except Exception as exc:
            messagebox.showerror("Ошибка", f"Не удалось открыть путь:\n{path}\n\n{exc}")

    def _open_desktop(self) -> None:
        self._open_path(desktop_path())

    def _open_last_result(self) -> None:
        if self.last_result_path and self.last_result_path.exists():
            # Selecting the zip in Explorer is nicer than opening it directly,
            # but `os.startfile` on a .zip will open it in the archive viewer
            # which is also fine. We keep behaviour simple and consistent.
            self._open_path(self.last_result_path)
        else:
            messagebox.showwarning("Нет результата", "Итоговый файл пока не создан.")

    def _reset_settings(self) -> None:
        if not messagebox.askyesno("Сброс", "Сбросить сохранённые настройки?"):
            return

        try:
            SETTINGS_FILE.unlink(missing_ok=True)
        except Exception:
            pass

        self.config = Config()
        self._load_config_to_ui()
        self._log("Настройки сброшены.")

    def _log(self, message: str) -> None:
        self.log_queue.put(message)

    def _poll_logs(self) -> None:
        try:
            while True:
                message = self.log_queue.get_nowait()
                timestamp = datetime.now().strftime("%H:%M:%S")
                self.log.configure(state="normal")
                self.log.insert("end", f"[{timestamp}] {message}\n")
                self.log.see("end")
                self.log.configure(state="disabled")
        except Empty:
            pass

        self.master.after(150, self._poll_logs)


def main() -> None:
    root = tk.Tk()

    try:
        style = ttk.Style()
        if "clam" in style.theme_names():
            style.theme_use("clam")
    except Exception:
        pass

    app = App(root)
    app._log(f"{APP_NAME} v{APP_VERSION} — готов к работе.")
    app._log("Выберите корневую папку проекта и нажмите «Создать экспорт».")
    root.mainloop()


if __name__ == "__main__":
    main()
