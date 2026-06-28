from __future__ import annotations

import json
import subprocess
from pathlib import Path

from project_exporter_desktop.services import export_history
from project_exporter_desktop.services.diff_service import (
    history_snapshot_payload,
    resolve_diff_selection,
)


def test_last_export_detects_added_modified_and_deleted(tmp_path: Path, monkeypatch) -> None:
    root = tmp_path / "project"
    root.mkdir()
    history_file = tmp_path / "history.json"
    monkeypatch.setattr(export_history, "HISTORY_FILE", history_file)

    (root / "a.py").write_text("print('one')", encoding="utf-8")
    (root / "gone.py").write_text("print('gone')", encoding="utf-8")
    previous = history_snapshot_payload(root, frozenset())
    history_file.write_text(
        json.dumps(
            [
                {
                    "source_root": str(root.resolve()),
                    "cancelled": False,
                    "snapshot": previous,
                }
            ],
            ensure_ascii=False,
        ),
        encoding="utf-8",
    )

    (root / "a.py").write_text("print('two')", encoding="utf-8")
    (root / "b.py").write_text("print('new')", encoding="utf-8")
    (root / "gone.py").unlink()

    selection = resolve_diff_selection(root, "last_export", ignored_dirs=frozenset())

    assert selection.paths == frozenset({"a.py", "b.py"})
    statuses = {(item.relative_path, item.status) for item in selection.files}
    assert ("a.py", "modified") in statuses
    assert ("b.py", "added") in statuses
    assert ("gone.py", "deleted") in statuses


def test_uncommitted_git_selection_includes_untracked(tmp_path: Path) -> None:
    root = tmp_path / "repo"
    root.mkdir()
    if subprocess.run(["git", "--version"], capture_output=True).returncode != 0:
        return
    subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "test@example.local"], cwd=root, check=True)
    subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
    (root / "tracked.py").write_text("print(1)", encoding="utf-8")
    subprocess.run(["git", "add", "tracked.py"], cwd=root, check=True)
    subprocess.run(["git", "commit", "-m", "init"], cwd=root, check=True, capture_output=True)

    (root / "tracked.py").write_text("print(2)", encoding="utf-8")
    (root / "new.py").write_text("print(3)", encoding="utf-8")

    selection = resolve_diff_selection(root, "uncommitted", ignored_dirs=frozenset())

    assert selection.paths == frozenset({"tracked.py", "new.py"})


def test_uncommitted_git_selection_handles_spaces(tmp_path: Path) -> None:
    root = tmp_path / "repo"
    root.mkdir()
    if subprocess.run(["git", "--version"], capture_output=True).returncode != 0:
        return
    subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "test@example.local"], cwd=root, check=True)
    subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)

    (root / "file with spaces.py").write_text("print(1)", encoding="utf-8")

    selection = resolve_diff_selection(root, "uncommitted", ignored_dirs=frozenset())

    assert selection.paths == frozenset({"file with spaces.py"})


def test_git_ref_selection_uses_head_not_working_tree(tmp_path: Path) -> None:
    root = tmp_path / "repo"
    root.mkdir()
    if subprocess.run(["git", "--version"], capture_output=True).returncode != 0:
        return
    subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "test@example.local"], cwd=root, check=True)
    subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
    (root / "tracked.py").write_text("print(1)", encoding="utf-8")
    subprocess.run(["git", "add", "tracked.py"], cwd=root, check=True)
    subprocess.run(["git", "commit", "-m", "init"], cwd=root, check=True, capture_output=True)
    base = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    (root / "committed.py").write_text("print(2)", encoding="utf-8")
    subprocess.run(["git", "add", "committed.py"], cwd=root, check=True)
    subprocess.run(["git", "commit", "-m", "second"], cwd=root, check=True, capture_output=True)
    (root / "tracked.py").write_text("print(3)", encoding="utf-8")

    selection = resolve_diff_selection(root, "git_ref", base, ignored_dirs=frozenset())

    assert selection.paths == frozenset({"committed.py"})
