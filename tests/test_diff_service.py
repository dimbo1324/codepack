"""Tests for the diff service: last-export, uncommitted, and git-ref selection modes."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

from project_exporter_desktop.services import export_history
from project_exporter_desktop.services.diff_service import (
    diff_manifest_payload,
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


def test_last_export_uses_latest_successful_snapshot_for_same_root(
    tmp_path: Path, monkeypatch
) -> None:
    root = tmp_path / "project"
    other_root = tmp_path / "other"
    root.mkdir()
    other_root.mkdir()
    history_file = tmp_path / "history.json"
    monkeypatch.setattr(export_history, "HISTORY_FILE", history_file)

    (root / "stable.py").write_text("print('old')", encoding="utf-8")
    old_snapshot = history_snapshot_payload(root, frozenset())
    (root / "stable.py").write_text("print('latest')", encoding="utf-8")
    latest_snapshot = history_snapshot_payload(root, frozenset())
    history_file.write_text(
        json.dumps(
            [
                {
                    "source_root": str(other_root.resolve()),
                    "cancelled": False,
                    "snapshot": {"noise.py": {"sha256": "x", "size": 1, "loc": 1}},
                },
                {
                    "source_root": str(root.resolve()),
                    "cancelled": True,
                    "snapshot": {"cancelled.py": {"sha256": "x", "size": 1, "loc": 1}},
                },
                {
                    "source_root": str(root.resolve()),
                    "cancelled": False,
                    "snapshot": latest_snapshot,
                },
                {
                    "source_root": str(root.resolve()),
                    "cancelled": False,
                    "snapshot": old_snapshot,
                },
            ],
            ensure_ascii=False,
        ),
        encoding="utf-8",
    )

    selection = resolve_diff_selection(root, "last_export", ignored_dirs=frozenset())

    assert selection.paths == frozenset()
    assert selection.files == ()


def test_last_export_without_history_selects_current_project_with_warning(
    tmp_path: Path, monkeypatch
) -> None:
    root = tmp_path / "project"
    root.mkdir()
    monkeypatch.setattr(export_history, "HISTORY_FILE", tmp_path / "missing_history.json")
    (root / "src").mkdir()
    (root / "src" / "main.py").write_text("print('ok')", encoding="utf-8")

    selection = resolve_diff_selection(root, "last_export", ignored_dirs=frozenset())

    assert selection.paths == frozenset({"src\\main.py"})
    assert selection.warning


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


def test_uncommitted_git_selection_handles_rename_with_spaces(tmp_path: Path) -> None:
    root = tmp_path / "repo"
    root.mkdir()
    if subprocess.run(["git", "--version"], capture_output=True).returncode != 0:
        return
    subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
    subprocess.run(["git", "config", "user.email", "test@example.local"], cwd=root, check=True)
    subprocess.run(["git", "config", "user.name", "Test"], cwd=root, check=True)
    old_path = root / "old name.py"
    old_path.write_text("print(1)", encoding="utf-8")
    subprocess.run(["git", "add", "old name.py"], cwd=root, check=True)
    subprocess.run(["git", "commit", "-m", "init"], cwd=root, check=True, capture_output=True)

    subprocess.run(["git", "mv", "old name.py", "new name.py"], cwd=root, check=True)

    selection = resolve_diff_selection(root, "uncommitted", ignored_dirs=frozenset())

    assert selection.paths == frozenset({"new name.py"})
    assert [(item.relative_path, item.old_path, item.status) for item in selection.files] == [
        ("new name.py", "old name.py", "renamed")
    ]


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


def test_diff_manifest_keeps_deleted_and_renamed_details(tmp_path: Path, monkeypatch) -> None:
    root = tmp_path / "project"
    root.mkdir()
    history_file = tmp_path / "history.json"
    monkeypatch.setattr(export_history, "HISTORY_FILE", history_file)
    (root / "keep.py").write_text("print('old')", encoding="utf-8")
    (root / "remove.py").write_text("print('remove')", encoding="utf-8")
    previous = history_snapshot_payload(root, frozenset())
    history_file.write_text(
        json.dumps(
            [{"source_root": str(root.resolve()), "cancelled": False, "snapshot": previous}],
            ensure_ascii=False,
        ),
        encoding="utf-8",
    )
    (root / "keep.py").write_text("print('new')", encoding="utf-8")
    (root / "remove.py").unlink()

    selection = resolve_diff_selection(root, "last_export", ignored_dirs=frozenset())
    payload = diff_manifest_payload(selection)

    assert payload is not None
    assert payload["selected_paths_count"] == 1
    assert {"path": "remove.py", "status": "deleted"} in payload["deleted_files"]
