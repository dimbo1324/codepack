from __future__ import annotations

from pathlib import Path

from project_exporter_desktop.services.export_ignore import ExportIgnoreRules


def _rules_from_content(tmp: Path, content: str) -> ExportIgnoreRules:
    (tmp / ".exportignore").write_text(content, encoding="utf-8")
    return ExportIgnoreRules.from_project_and_config(tmp)


def test_empty_project_no_rules(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(tmp_path)
    skip, _ = rules.should_skip_file(Path("any/file.py"))
    assert skip is False


def test_explicit_file_exclude(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "secrets.txt\n")
    skip, reason = rules.should_skip_file(Path("secrets.txt"))
    assert skip is True
    assert reason != ""


def test_negation_overrides_glob(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "*.log\n!important.log\n")
    assert rules.should_skip_file(Path("debug.log"))[0] is True
    assert rules.should_skip_file(Path("important.log"))[0] is False


def test_directory_rule(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "build/\n")
    assert rules.should_skip_file(Path("build/output.o"))[0] is True
    assert rules.should_skip_file(Path("src/main.py"))[0] is False


def test_directory_skip_dir(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "build/\n")
    skip, _ = rules.should_skip_dir(Path("build"))
    assert skip is True


def test_double_star_as_file_glob(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "**/node_modules/**\n")
    assert rules.should_skip_file(Path("frontend/node_modules/react/index.js"))[0] is True
    assert rules.should_skip_file(Path("frontend/src/index.js"))[0] is False


def test_add_always_include_overrides_exclusion(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "*.secret\n")
    rules.add_always_include_file("shared.secret")
    assert rules.should_skip_file(Path("shared.secret"))[0] is False
    assert rules.should_skip_file(Path("other.secret"))[0] is True


def test_add_file_rule_excludes_specific(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(tmp_path)
    rules.add_file_rule("src/generated/types.ts")
    assert rules.should_skip_file(Path("src/generated/types.ts"))[0] is True
    assert rules.should_skip_file(Path("src/components/App.tsx"))[0] is False


def test_conflicting_always_include_wins_over_file_rule(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(tmp_path)
    rules.add_file_rule("config.yml")
    rules.add_always_include_file("config.yml")
    assert rules.should_skip_file(Path("config.yml"))[0] is False


def test_comment_lines_ignored(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "# this is a comment\n*.pyc\n# another comment\n")
    assert rules.should_skip_file(Path("module.pyc"))[0] is True
    assert rules.should_skip_file(Path("module.py"))[0] is False


def test_blank_lines_ignored(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "\n\n*.tmp\n\n")
    assert rules.should_skip_file(Path("work.tmp"))[0] is True
    assert rules.should_skip_file(Path("work.txt"))[0] is False


def test_from_project_with_excluded_files_config(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(
        tmp_path,
        excluded_files=["private.key", "*.pem"],
        excluded_extensions=[],
        always_include_files=[],
        always_include_dirs=[],
    )
    assert rules.should_skip_file(Path("private.key"))[0] is True
    assert rules.should_skip_file(Path("cert.pem"))[0] is True
    assert rules.should_skip_file(Path("readme.md"))[0] is False


def test_from_project_with_always_include_config(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(
        tmp_path,
        excluded_files=["*.log"],
        excluded_extensions=[],
        always_include_files=["app.log"],
        always_include_dirs=[],
    )
    assert rules.should_skip_file(Path("app.log"))[0] is False
    assert rules.should_skip_file(Path("debug.log"))[0] is True


def test_extension_exclusion(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(
        tmp_path,
        excluded_files=[],
        excluded_extensions=[".exe", ".dll"],
        always_include_files=[],
        always_include_dirs=[],
    )
    assert rules.should_skip_file(Path("program.exe"))[0] is True
    assert rules.should_skip_file(Path("library.dll"))[0] is True
    assert rules.should_skip_file(Path("script.py"))[0] is False


def test_unicode_filename(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "секрет.txt\n")
    assert rules.should_skip_file(Path("секрет.txt"))[0] is True
    assert rules.should_skip_file(Path("публичный.txt"))[0] is False


def test_deeply_nested_path(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(tmp_path)
    rules.add_file_rule("a/b/c/d/secret.txt")
    assert rules.should_skip_file(Path("a/b/c/d/secret.txt"))[0] is True
    assert rules.should_skip_file(Path("a/b/c/d/public.txt"))[0] is False


def test_path_with_backslash_normalized(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(tmp_path)
    rules.add_file_rule("src\\utils\\helper.py")
    assert rules.should_skip_file(Path("src/utils/helper.py"))[0] is True


def test_should_skip_file_returns_reason_string(tmp_path: Path) -> None:
    rules = _rules_from_content(tmp_path, "*.secret\n")
    skip, reason = rules.should_skip_file(Path("key.secret"))
    assert skip is True
    assert isinstance(reason, str)
    assert len(reason) > 0


def test_always_include_dir_protects_children(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(tmp_path)
    rules.add_dir_rule("internal")
    rules.add_always_include_dir("internal/public")
    skip_dir, _ = rules.should_skip_dir(Path("internal/public"))
    assert skip_dir is False


def test_no_rule_for_unknown_extension(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(tmp_path)
    assert rules.should_skip_file(Path("file.xyz123unknown"))[0] is False


def test_extension_without_dot_in_config(tmp_path: Path) -> None:
    rules = ExportIgnoreRules.from_project_and_config(
        tmp_path,
        excluded_extensions=["pyc"],
        excluded_files=[],
        always_include_files=[],
        always_include_dirs=[],
    )
    assert rules.should_skip_file(Path("cache/__pycache__/module.pyc"))[0] is True
