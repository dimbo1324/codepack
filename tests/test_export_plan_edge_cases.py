from __future__ import annotations

from pathlib import Path

from project_exporter_desktop.config import Config
from project_exporter_desktop.services.diff_service import DiffSelection
from project_exporter_desktop.services.export_ignore import ExportIgnoreRules
from project_exporter_desktop.services.export_plan import build_export_plan
from project_exporter_desktop.services.incremental import IncrementalSelection

_DIFF_ALL = DiffSelection(mode="all", base="HEAD", paths=None)
_INCREMENTAL_OFF = IncrementalSelection(enabled=False)


def _plan(
    tmp_path: Path,
    config: Config | None = None,
    diff_selection: DiffSelection | None = None,
    incremental: IncrementalSelection | None = None,
    overrides: ExportIgnoreRules | None = None,
):
    if config is None:
        config = Config()
    if diff_selection is None:
        diff_selection = _DIFF_ALL
    if incremental is None:
        incremental = _INCREMENTAL_OFF
    if overrides is None:
        overrides = ExportIgnoreRules.from_project_and_config(tmp_path)
    ignored = config.effective_ignored_dirs()
    return build_export_plan(tmp_path, config, ignored, diff_selection, incremental, overrides)


def _touch(tmp_path: Path, *names: str) -> None:
    for name in names:
        p = tmp_path / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(name, encoding="utf-8")


def test_empty_project(tmp_path: Path) -> None:
    plan = _plan(tmp_path)
    assert plan.included_count == 0
    assert plan.excluded_count == 0


def test_single_python_file(tmp_path: Path) -> None:
    _touch(tmp_path, "main.py")
    plan = _plan(tmp_path)
    assert plan.included_count >= 1


def test_nested_directory_included(tmp_path: Path) -> None:
    _touch(tmp_path, "src/core/utils.py", "src/core/models.py", "README.md")
    plan = _plan(tmp_path)
    assert plan.included_count == 3


def test_exportignore_excludes_log_files(tmp_path: Path) -> None:
    _touch(tmp_path, "main.py", "debug.log")
    (tmp_path / ".exportignore").write_text("*.log\n", encoding="utf-8")
    rules = ExportIgnoreRules.from_project_and_config(tmp_path)
    plan = _plan(tmp_path, overrides=rules)
    included_names = {pf.relative_path for pf in plan.included_files}
    assert any("main.py" in n for n in included_names)
    assert all("debug.log" not in n for n in included_names)


def test_ignored_dir_excluded_by_config(tmp_path: Path) -> None:
    _touch(tmp_path, "main.py")
    node_modules = tmp_path / "node_modules"
    node_modules.mkdir()
    (node_modules / "package.json").write_text("{}", encoding="utf-8")
    config = Config(extra_ignored_dirs=["node_modules"])
    plan = _plan(tmp_path, config=config)
    paths = {pf.relative_path for pf in plan.included_files}
    assert not any("node_modules" in p for p in paths)


def test_diff_mode_restricts_to_selected_files(tmp_path: Path) -> None:
    _touch(tmp_path, "changed.py", "unchanged.py")
    diff = DiffSelection(
        mode="git_ref",
        base="HEAD",
        paths=frozenset({"changed.py"}),
    )
    plan = _plan(tmp_path, diff_selection=diff)
    included = {pf.relative_path for pf in plan.included_files}
    assert any("changed.py" in p for p in included)
    assert all("unchanged.py" not in p for p in included)


def test_plan_counts_are_consistent(tmp_path: Path) -> None:
    _touch(tmp_path, "a.py", "b.py", "c.py")
    plan = _plan(tmp_path)
    total = plan.included_count + plan.excluded_count
    assert total == len(plan.included_files) + len(plan.excluded_files)


def test_estimated_bytes_nonnegative(tmp_path: Path) -> None:
    _touch(tmp_path, "code.py")
    plan = _plan(tmp_path)
    assert plan.estimated_included_bytes >= 0


def test_estimated_bytes_matches_file_sizes(tmp_path: Path) -> None:
    content = "x" * 1000
    (tmp_path / "file.py").write_text(content, encoding="utf-8")
    plan = _plan(tmp_path)
    total_bytes = sum(pf.size for pf in plan.included_files)
    assert total_bytes == plan.estimated_included_bytes


def test_always_include_overrides_exportignore(tmp_path: Path) -> None:
    (tmp_path / ".exportignore").write_text("*.log\n", encoding="utf-8")
    _touch(tmp_path, "app.log", "main.py")
    rules = ExportIgnoreRules.from_project_and_config(tmp_path)
    rules.add_always_include_file("app.log")
    plan = _plan(tmp_path, overrides=rules)
    included = {pf.relative_path for pf in plan.included_files}
    assert any("app.log" in p for p in included)


def test_unicode_filename_handled(tmp_path: Path) -> None:
    _touch(tmp_path, "кириллица.py", "readme.md")
    plan = _plan(tmp_path)
    assert plan.included_count >= 1


def test_deeply_nested_files(tmp_path: Path) -> None:
    _touch(tmp_path, "a/b/c/d/e/f/deep.py")
    plan = _plan(tmp_path)
    assert plan.included_count >= 1
    included = {pf.relative_path for pf in plan.included_files}
    assert any("deep.py" in p for p in included)


def test_very_long_filename(tmp_path: Path) -> None:
    long_name = "x" * 200 + ".py"
    _touch(tmp_path, long_name)
    plan = _plan(tmp_path)
    assert plan.included_count >= 1


def test_file_with_no_extension(tmp_path: Path) -> None:
    _touch(tmp_path, "Makefile", "Dockerfile", "main.py")
    plan = _plan(tmp_path)
    assert plan.included_count >= 1


def test_empty_file_handled(tmp_path: Path) -> None:
    (tmp_path / "empty.py").write_text("", encoding="utf-8")
    plan = _plan(tmp_path)
    assert plan.included_count == 1
    assert plan.estimated_included_bytes == 0


def test_plan_project_name_from_dir(tmp_path: Path) -> None:
    _touch(tmp_path, "main.py")
    plan = _plan(tmp_path)
    assert plan.project_name == tmp_path.name


def test_plan_warnings_list_is_list(tmp_path: Path) -> None:
    plan = _plan(tmp_path)
    assert isinstance(plan.warnings, list)


def test_diff_mode_none_includes_all(tmp_path: Path) -> None:
    _touch(tmp_path, "a.py", "b.py")
    diff = DiffSelection(mode="all", base="HEAD", paths=None)
    plan = _plan(tmp_path, diff_selection=diff)
    assert plan.included_count == 2


def test_diff_mode_empty_frozenset_excludes_all(tmp_path: Path) -> None:
    _touch(tmp_path, "a.py", "b.py")
    diff = DiffSelection(mode="git_ref", base="HEAD", paths=frozenset())
    plan = _plan(tmp_path, diff_selection=diff)
    assert plan.included_count == 0


def test_sensitive_warnings_returns_high_severity(tmp_path: Path) -> None:
    plan = _plan(tmp_path)
    for item in plan.sensitive_warnings:
        assert item.severity in {"critical", "high"}
