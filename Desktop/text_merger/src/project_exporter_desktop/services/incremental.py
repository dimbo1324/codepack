from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from pathlib import Path

from ..constants import SETTINGS_FILE
from ..utils.path_utils import should_ignore_dir

STATE_FILE = SETTINGS_FILE.with_name('.project_exporter_incremental_state.json')


@dataclass(slots=True)
class IncrementalSelection:
    enabled: bool = False
    paths: frozenset[str] | None = None
    added: list[str] = field(default_factory=list)
    modified: list[str] = field(default_factory=list)
    deleted: list[str] = field(default_factory=list)
    unchanged: int = 0
    warning: str | None = None

    @property
    def is_limited(self) -> bool:
        return self.enabled and self.paths is not None


def _load_state() -> dict[str, object]:
    try:
        if STATE_FILE.exists():
            return json.loads(STATE_FILE.read_text(encoding='utf-8'))
    except Exception:
        return {}
    return {}


def _write_state(data: dict[str, object]) -> None:
    try:
        STATE_FILE.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding='utf-8', newline='\n')
    except Exception:
        pass


def _snapshot_project(root: Path, ignored_dirs: frozenset[str] | set[str]) -> dict[str, dict[str, int]]:
    result: dict[str, dict[str, int]] = {}
    for current_dir, dirnames, filenames in os.walk(root, topdown=True, followlinks=False):
        current = Path(current_dir)
        dirnames[:] = [
            dirname for dirname in dirnames
            if not should_ignore_dir(dirname, ignored_dirs) and not (current / dirname).is_symlink()
        ]
        for filename in filenames:
            path = current / filename
            if path.is_symlink():
                continue
            try:
                rel = str(path.relative_to(root)).replace('/', '\\')
                st = path.stat()
                result[rel] = {'size': int(st.st_size), 'mtime_ns': int(st.st_mtime_ns)}
            except Exception:
                continue
    return result


def resolve_incremental_selection(root: Path, ignored_dirs: frozenset[str] | set[str], enabled: bool) -> IncrementalSelection:
    if not enabled:
        return IncrementalSelection(enabled=False)
    try:
        state = _load_state()
        project_key = str(root.resolve())
        previous = state.get(project_key)
        current = _snapshot_project(root, ignored_dirs)
        if not isinstance(previous, dict):
            return IncrementalSelection(
                enabled=True,
                paths=frozenset(current),
                added=sorted(current),
                warning='No previous incremental state for this project; exporting all currently visible files and saving baseline.',
            )
        added: list[str] = []
        modified: list[str] = []
        unchanged = 0
        for rel, meta in current.items():
            old = previous.get(rel)
            if not isinstance(old, dict):
                added.append(rel)
            elif old.get('size') != meta.get('size') or old.get('mtime_ns') != meta.get('mtime_ns'):
                modified.append(rel)
            else:
                unchanged += 1
        deleted = sorted(str(rel) for rel in previous if rel not in current)
        selected = frozenset(sorted(added + modified))
        return IncrementalSelection(
            enabled=True,
            paths=selected,
            added=sorted(added),
            modified=sorted(modified),
            deleted=deleted,
            unchanged=unchanged,
        )
    except Exception as exc:
        return IncrementalSelection(enabled=True, warning=f'Incremental state failed: {type(exc).__name__}: {exc}')


def save_incremental_baseline(root: Path, ignored_dirs: frozenset[str] | set[str]) -> None:
    try:
        state = _load_state()
        state[str(root.resolve())] = _snapshot_project(root, ignored_dirs)
        _write_state(state)
    except Exception:
        pass


def write_export_diff_report(output_file: Path, selection: IncrementalSelection) -> None:
    output_file.parent.mkdir(parents=True, exist_ok=True)
    with output_file.open('w', encoding='utf-8', newline='\n') as out:
        out.write('# Export Comparison Report\n\n')
        if not selection.enabled:
            out.write('Incremental export is disabled. No previous-export comparison was requested.\n')
            return
        if selection.warning:
            out.write(f'Warning: {selection.warning}\n\n')
        out.write(f'- Added files: {len(selection.added):,}\n')
        out.write(f'- Modified files: {len(selection.modified):,}\n')
        out.write(f'- Deleted since previous baseline: {len(selection.deleted):,}\n')
        out.write(f'- Unchanged files: {selection.unchanged:,}\n\n')
        for title, items in [('Added', selection.added), ('Modified', selection.modified), ('Deleted', selection.deleted)]:
            out.write(f'## {title}\n\n')
            if not items:
                out.write('- none\n\n')
                continue
            for rel in items[:500]:
                out.write(f'- `{rel}`\n')
            if len(items) > 500:
                out.write(f'- ... and {len(items) - 500:,} more\n')
            out.write('\n')
