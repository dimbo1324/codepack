from __future__ import annotations

import hashlib
import os
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from ..constants import TEXT_EXTENSIONS, TEXT_FILENAMES_WITHOUT_EXTENSION
from ..utils.path_utils import should_ignore_dir
from ..utils.text_utils import format_bytes
from .export_history import load_export_history


@dataclass(frozen=True, slots=True)
class DiffFile:
    relative_path: str
    status: str
    old_path: str = ""


@dataclass(frozen=True, slots=True)
class DiffSelection:
    mode: str
    base: str
    paths: frozenset[str] | None
    files: tuple[DiffFile, ...] = field(default_factory=tuple)
    warning: str | None = None

    @property
    def is_limited(self) -> bool:
        return self.paths is not None

    @property
    def deleted(self) -> tuple[DiffFile, ...]:
        return tuple(item for item in self.files if item.status == "deleted")


def _normalise_rel(path: str | Path) -> str:
    return str(path).strip().strip('"').replace("/", "\\")


def _run_git(args: list[str], cwd: Path) -> tuple[int, list[str], str]:
    try:
        completed = subprocess.run(
            ["git", *args],
            cwd=str(cwd),
            text=True,
            capture_output=True,
            timeout=90,
            shell=False,
            encoding="utf-8",
            errors="replace",
        )
        lines = [line.rstrip("\n") for line in completed.stdout.splitlines() if line.strip()]
        return completed.returncode, lines, completed.stderr.strip()
    except FileNotFoundError:
        return 127, [], "Git не найден в системе."
    except Exception as exc:
        return 1, [], f"{type(exc).__name__}: {exc}"


def _hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _count_loc(path: Path) -> int:
    suffix = path.suffix.casefold().lstrip(".")
    if suffix not in TEXT_EXTENSIONS and path.name.casefold() not in TEXT_FILENAMES_WITHOUT_EXTENSION:
        return 0
    try:
        return sum(
            1
            for line in path.read_text(encoding="utf-8", errors="replace").splitlines()
            if line.strip()
        )
    except Exception:
        return 0


def snapshot_project(root: Path, ignored_dirs: frozenset[str] | set[str]) -> dict[str, dict[str, Any]]:
    snapshot: dict[str, dict[str, Any]] = {}
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
                rel = _normalise_rel(path.relative_to(root))
                stat = path.stat()
                snapshot[rel] = {
                    "sha256": _hash_file(path),
                    "size": int(stat.st_size),
                    "loc": _count_loc(path),
                    "mtime_ns": int(stat.st_mtime_ns),
                }
            except Exception:
                continue
    return snapshot


def _last_history_snapshot(root: Path) -> dict[str, dict[str, Any]] | None:
    project_key = str(root.resolve())
    for entry in load_export_history():
        if str(entry.get("source_root", "")) != project_key:
            continue
        if bool(entry.get("cancelled")):
            continue
        snapshot = entry.get("snapshot")
        if isinstance(snapshot, dict):
            return {
                str(path): dict(meta)
                for path, meta in snapshot.items()
                if isinstance(path, str) and isinstance(meta, dict)
            }
    return None


def _selection_from_snapshots(
    mode: str,
    base: str,
    previous: dict[str, dict[str, Any]] | None,
    current: dict[str, dict[str, Any]],
) -> DiffSelection:
    if previous is None:
        files = tuple(DiffFile(path, "added") for path in sorted(current))
        return DiffSelection(
            mode=mode,
            base=base,
            paths=frozenset(current),
            files=files,
            warning=(
                "Предыдущий экспорт для проекта не найден, поэтому выбран весь текущий проект."
            ),
        )

    changed: list[DiffFile] = []
    for rel, meta in current.items():
        old = previous.get(rel)
        if old is None:
            changed.append(DiffFile(rel, "added"))
        elif old.get("sha256") != meta.get("sha256"):
            changed.append(DiffFile(rel, "modified"))
    for rel in previous:
        if rel not in current:
            changed.append(DiffFile(rel, "deleted"))

    selected = frozenset(item.relative_path for item in changed if item.status != "deleted")
    return DiffSelection(
        mode=mode,
        base=base,
        paths=selected,
        files=tuple(sorted(changed, key=lambda item: item.relative_path)),
    )


def _parse_name_status(lines: list[str]) -> tuple[DiffFile, ...]:
    files: list[DiffFile] = []
    for line in lines:
        parts = line.split("\t")
        if len(parts) < 2:
            continue
        status_code = parts[0].strip()
        status_letter = status_code[:1].upper()
        if status_letter == "D":
            files.append(DiffFile(_normalise_rel(parts[1]), "deleted"))
        elif status_letter == "A":
            files.append(DiffFile(_normalise_rel(parts[-1]), "added"))
        elif status_letter == "R" and len(parts) >= 3:
            files.append(
                DiffFile(
                    _normalise_rel(parts[2]),
                    "renamed",
                    _normalise_rel(parts[1]),
                )
            )
        else:
            files.append(DiffFile(_normalise_rel(parts[-1]), "modified"))
    return tuple(files)


def _parse_porcelain(lines: list[str]) -> tuple[DiffFile, ...]:
    files: list[DiffFile] = []
    for line in lines:
        if len(line) < 4:
            continue
        status = line[:2]
        raw_path = line[3:].strip()
        if " -> " in raw_path:
            old_path, new_path = raw_path.split(" -> ", 1)
            files.append(DiffFile(_normalise_rel(new_path), "renamed", _normalise_rel(old_path)))
            continue
        rel = _normalise_rel(raw_path)
        if "D" in status:
            files.append(DiffFile(rel, "deleted"))
        elif status == "??":
            files.append(DiffFile(rel, "added"))
        else:
            files.append(DiffFile(rel, "modified"))
    return tuple(files)


def _disabled_by_git_error(message: str) -> DiffSelection:
    return DiffSelection(
        "all",
        "полный экспорт",
        None,
        warning=f"Дифференциальный Git-режим отключён: {message}",
    )


def _git_selection(source_root: Path, mode: str, base_ref: str) -> DiffSelection:
    rc, _inside, err = _run_git(["rev-parse", "--is-inside-work-tree"], source_root)
    if rc != 0:
        return _disabled_by_git_error(err or "это не Git-репозиторий")

    if mode == "uncommitted":
        rc, lines, err = _run_git(["status", "--porcelain"], source_root)
        if rc != 0:
            return _disabled_by_git_error(err or "git status завершился ошибкой")
        files = _parse_porcelain(lines)
        selected = frozenset(item.relative_path for item in files if item.status != "deleted")
        return DiffSelection(mode, "незакоммиченные изменения", selected, files)

    base = (base_ref or "HEAD").strip()
    rc, lines, err = _run_git(["diff", "--name-status", "--find-renames", base, "--"], source_root)
    if rc != 0:
        return _disabled_by_git_error(err or "git diff завершился ошибкой")
    files = _parse_name_status(lines)
    selected = frozenset(item.relative_path for item in files if item.status != "deleted")
    return DiffSelection(mode, base, selected, files)


def resolve_diff_selection(
    source_root: Path,
    mode: str,
    base_ref: str = "HEAD",
    _target_ref: str = "",
    ignored_dirs: frozenset[str] | set[str] = frozenset(),
) -> DiffSelection:
    if mode == "all":
        return DiffSelection("all", "полный экспорт", None)
    if mode == "last_export":
        current = snapshot_project(source_root, ignored_dirs)
        previous = _last_history_snapshot(source_root)
        return _selection_from_snapshots("last_export", "последний экспорт", previous, current)
    if mode in {"git_ref", "uncommitted"}:
        return _git_selection(source_root, mode, base_ref)
    return DiffSelection(
        "all",
        "полный экспорт",
        None,
        warning=f"Неизвестный режим дифференциального экспорта: {mode}",
    )


def diff_manifest_payload(selection: DiffSelection | None) -> dict[str, Any] | None:
    if selection is None:
        return None
    return {
        "mode": selection.mode,
        "diff_base": selection.base,
        "limited": selection.is_limited,
        "selected_paths_count": len(selection.paths) if selection.paths is not None else None,
        "warning": selection.warning,
        "files": [
            {
                "path": item.relative_path,
                "status": item.status,
                **({"old_path": item.old_path} if item.old_path else {}),
            }
            for item in selection.files
        ],
        "deleted_files": [
            {"path": item.relative_path, "status": "deleted"} for item in selection.deleted
        ],
    }


def write_diff_report(output_file: Path, selection: DiffSelection) -> None:
    output_file.parent.mkdir(parents=True, exist_ok=True)
    changed = [item for item in selection.files if item.status != "deleted"]
    with output_file.open("w", encoding="utf-8", newline="\n") as out:
        out.write("# Дифференциальный экспорт\n\n")
        out.write(f"- Режим: {selection.mode}\n")
        out.write(f"- База: {selection.base}\n")
        out.write(f"- Ограничение набора файлов: {'да' if selection.is_limited else 'нет'}\n")
        out.write(f"- Изменённых файлов в архиве: {len(changed):,}\n")
        out.write(f"- Удалённых файлов в manifest: {len(selection.deleted):,}\n\n")
        if selection.warning:
            out.write(f"Предупреждение: {selection.warning}\n\n")
        for status, title in [
            ("added", "Добавленные"),
            ("modified", "Изменённые"),
            ("renamed", "Переименованные"),
            ("deleted", "Удалённые"),
        ]:
            items = [item for item in selection.files if item.status == status]
            out.write(f"## {title}\n\n")
            if not items:
                out.write("- нет\n\n")
                continue
            for item in items[:500]:
                old = f" (ранее: `{item.old_path}`)" if item.old_path else ""
                out.write(f"- `{item.relative_path}`{old}\n")
            if len(items) > 500:
                out.write(f"- ... и ещё {len(items) - 500:,}\n")
            out.write("\n")


def history_snapshot_payload(
    root: Path, ignored_dirs: frozenset[str] | set[str]
) -> dict[str, dict[str, Any]]:
    snapshot = snapshot_project(root, ignored_dirs)
    return {
        rel: {
            "sha256": meta.get("sha256", ""),
            "size": int(meta.get("size", 0)),
            "loc": int(meta.get("loc", 0)),
        }
        for rel, meta in snapshot.items()
    }


def snapshot_stats(snapshot: dict[str, dict[str, Any]]) -> dict[str, Any]:
    size = sum(int(meta.get("size", 0)) for meta in snapshot.values())
    return {"files": len(snapshot), "bytes": size, "size_human": format_bytes(size)}
