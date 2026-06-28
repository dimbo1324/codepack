"""Tests for the stack detector: identifies project technology stacks from marker files."""

from __future__ import annotations

from pathlib import Path

from project_exporter_desktop.services.stack_detector import (
    StackInfo,
    detect_stack,
    format_stack_label,
    merged_extra_ignored_dirs,
    primary_stack,
)


def _make_files(tmp_path: Path, *names: str) -> Path:
    for name in names:
        (tmp_path / name).touch()
    return tmp_path


def test_detect_nodejs(tmp_path: Path) -> None:
    _make_files(tmp_path, "package.json")
    stacks = detect_stack(tmp_path)
    assert any(s.name == "Node.js" for s in stacks)
    node = next(s for s in stacks if s.name == "Node.js")
    assert "package.json" in node.markers_found
    assert "node_modules" in node.extra_ignored_dirs


def test_detect_python_requirements(tmp_path: Path) -> None:
    _make_files(tmp_path, "requirements.txt")
    stacks = detect_stack(tmp_path)
    assert any(s.name == "Python" for s in stacks)


def test_detect_python_pyproject(tmp_path: Path) -> None:
    _make_files(tmp_path, "pyproject.toml")
    stacks = detect_stack(tmp_path)
    assert any(s.name == "Python" for s in stacks)


def test_detect_go(tmp_path: Path) -> None:
    _make_files(tmp_path, "go.mod")
    stacks = detect_stack(tmp_path)
    assert any(s.name == "Go" for s in stacks)
    go = next(s for s in stacks if s.name == "Go")
    assert "vendor" in go.extra_ignored_dirs


def test_detect_rust(tmp_path: Path) -> None:
    _make_files(tmp_path, "Cargo.toml")
    stacks = detect_stack(tmp_path)
    assert any(s.name == "Rust" for s in stacks)
    rust = next(s for s in stacks if s.name == "Rust")
    assert "target" in rust.extra_ignored_dirs


def test_detect_flutter(tmp_path: Path) -> None:
    _make_files(tmp_path, "pubspec.yaml")
    stacks = detect_stack(tmp_path)
    assert any(s.name == "Flutter / Dart" for s in stacks)


def test_detect_php(tmp_path: Path) -> None:
    _make_files(tmp_path, "composer.json")
    stacks = detect_stack(tmp_path)
    assert any(s.name == "PHP / Composer" for s in stacks)


def test_detect_ruby(tmp_path: Path) -> None:
    _make_files(tmp_path, "Gemfile")
    stacks = detect_stack(tmp_path)
    assert any(s.name == "Ruby" for s in stacks)


def test_detect_unknown_stack(tmp_path: Path) -> None:
    _make_files(tmp_path, "README.md", "main.c")
    stacks = detect_stack(tmp_path)
    assert stacks == []


def test_detect_monorepo(tmp_path: Path) -> None:
    _make_files(tmp_path, "package.json", "requirements.txt")
    stacks = detect_stack(tmp_path)
    names = {s.name for s in stacks}
    assert "Node.js" in names
    assert "Python" in names


def test_primary_stack_returns_most_confident(tmp_path: Path) -> None:
    _make_files(tmp_path, "package.json")
    info = primary_stack(tmp_path)
    assert info is not None
    assert info.name == "Node.js"


def test_primary_stack_empty_dir(tmp_path: Path) -> None:
    assert primary_stack(tmp_path) is None


def test_merged_extra_ignored_dirs_single_stack(tmp_path: Path) -> None:
    _make_files(tmp_path, "package.json")
    dirs = merged_extra_ignored_dirs(tmp_path)
    assert "node_modules" in dirs


def test_merged_extra_ignored_dirs_multiple_stacks(tmp_path: Path) -> None:
    _make_files(tmp_path, "package.json", "requirements.txt")
    dirs = merged_extra_ignored_dirs(tmp_path)
    assert "node_modules" in dirs
    assert ".venv" in dirs


def test_merged_extra_ignored_dirs_no_stack(tmp_path: Path) -> None:
    dirs = merged_extra_ignored_dirs(tmp_path)
    assert len(dirs) == 0


def test_format_stack_label_single(tmp_path: Path) -> None:
    _make_files(tmp_path, "go.mod")
    label = format_stack_label(tmp_path)
    assert "Go" in label
    assert "go.mod" in label


def test_format_stack_label_multiple(tmp_path: Path) -> None:
    _make_files(tmp_path, "package.json", "pyproject.toml")
    label = format_stack_label(tmp_path)
    assert "Node.js" in label
    assert "Python" in label


def test_format_stack_label_unknown(tmp_path: Path) -> None:
    label = format_stack_label(tmp_path)
    assert label == ""


def test_detect_nonexistent_dir(tmp_path: Path) -> None:
    missing = tmp_path / "does_not_exist"
    assert detect_stack(missing) == []


def test_stack_info_display_label() -> None:
    info = StackInfo(
        name="Node.js",
        markers_found=("package.json",),
        extra_ignored_dirs=frozenset({"node_modules"}),
    )
    label = info.display_label()
    assert "Node.js" in label
    assert "package.json" in label
