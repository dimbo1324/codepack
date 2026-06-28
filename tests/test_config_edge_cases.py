from __future__ import annotations

import json
from pathlib import Path

import pytest

import project_exporter_desktop.config as _config_module
from project_exporter_desktop.config import Config, _migrate_legacy_settings


def test_load_with_unknown_keys(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    cfg_file = tmp_path / "settings.json"
    data = {"last_root": str(tmp_path), "unknown_future_key": "value", "another_unknown": 42}
    cfg_file.write_text(json.dumps(data), encoding="utf-8")
    monkeypatch.setattr(_config_module, "SETTINGS_FILE", cfg_file)
    cfg = Config.load()
    assert cfg.last_root == str(tmp_path)


def test_load_with_empty_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    cfg_file = tmp_path / "settings.json"
    cfg_file.write_text("{}", encoding="utf-8")
    monkeypatch.setattr(_config_module, "SETTINGS_FILE", cfg_file)
    cfg = Config.load()
    assert cfg.redact_secrets is True
    assert cfg.ui_zoom == 1.0


def test_load_with_corrupt_json(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    cfg_file = tmp_path / "settings.json"
    cfg_file.write_text("NOT JSON {{{{", encoding="utf-8")
    monkeypatch.setattr(_config_module, "SETTINGS_FILE", cfg_file)
    cfg = Config.load()
    assert cfg.redact_secrets is True


def test_load_with_nonexistent_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    cfg_file = tmp_path / "doesnotexist.json"
    monkeypatch.setattr(_config_module, "SETTINGS_FILE", cfg_file)
    cfg = Config.load()
    assert cfg.redact_secrets is True
    assert isinstance(cfg, Config)


def test_normalized_ui_zoom_clamps_high() -> None:
    cfg = Config(ui_zoom=3.0)
    assert cfg.normalized_ui_zoom() == 1.5


def test_normalized_ui_zoom_clamps_low() -> None:
    cfg = Config(ui_zoom=0.1)
    assert cfg.normalized_ui_zoom() == 0.7


def test_normalized_ui_zoom_default() -> None:
    cfg = Config()
    assert cfg.normalized_ui_zoom() == 1.0


def test_normalized_ui_zoom_mid_value() -> None:
    cfg = Config(ui_zoom=1.2)
    assert cfg.normalized_ui_zoom() == pytest.approx(1.2)


def test_normalized_ui_zoom_invalid_type() -> None:
    cfg = Config()
    object.__setattr__(cfg, "ui_zoom", "bad_string")
    assert cfg.normalized_ui_zoom() == 1.0


def test_normalized_ui_zoom_none() -> None:
    cfg = Config()
    object.__setattr__(cfg, "ui_zoom", None)
    assert cfg.normalized_ui_zoom() == 1.0


def test_normalized_theme_invalid() -> None:
    cfg = Config(theme="neon")
    assert cfg.normalized_theme() == "system"


def test_normalized_theme_valid_values() -> None:
    for valid in ("system", "light", "dark"):
        cfg = Config(theme=valid)
        assert cfg.normalized_theme() == valid


def test_normalized_export_profile_invalid() -> None:
    from project_exporter_desktop.constants import DEFAULT_EXPORT_PROFILE

    cfg = Config(export_profile="nonexistent_profile")
    assert cfg.normalized_export_profile() == DEFAULT_EXPORT_PROFILE


def test_save_and_reload_roundtrip(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    cfg_file = tmp_path / "settings.json"
    monkeypatch.setattr(_config_module, "SETTINGS_FILE", cfg_file)
    cfg = Config(ui_zoom=1.3, theme="dark", developer_context="hello world")
    cfg.save()
    loaded = Config.load()
    assert loaded.ui_zoom == pytest.approx(1.3)
    assert loaded.theme == "dark"
    assert loaded.developer_context == "hello world"


def test_migrate_legacy_max_text_file_mb_nondefault() -> None:
    data = {"max_text_file_mb": 10}
    result = _migrate_legacy_settings(data)
    assert result["text_file_size_limit_enabled"] is True


def test_migrate_legacy_max_text_file_mb_default() -> None:
    data = {"max_text_file_mb": 5}
    result = _migrate_legacy_settings(data)
    assert result["text_file_size_limit_enabled"] is False


def test_migrate_skips_if_already_present() -> None:
    data = {"max_text_file_mb": 10, "text_file_size_limit_enabled": False}
    result = _migrate_legacy_settings(data)
    assert result["text_file_size_limit_enabled"] is False


def test_migrate_ignores_data_without_max_text_key() -> None:
    data = {"last_root": "/home"}
    result = _migrate_legacy_settings(data)
    assert "text_file_size_limit_enabled" not in result


def test_effective_ignored_dirs_includes_extras() -> None:
    cfg = Config(extra_ignored_dirs=["MySpecialDir", "AnotherDir"])
    result = cfg.effective_ignored_dirs()
    assert "myspecialdir" in result
    assert "anotherdir" in result


def test_effective_ignored_dirs_strips_whitespace() -> None:
    cfg = Config(extra_ignored_dirs=["  spaced  ", "\ttabbed\t"])
    result = cfg.effective_ignored_dirs()
    assert "spaced" in result
    assert "tabbed" in result


def test_effective_ignored_dirs_deduplicates() -> None:
    cfg = Config(extra_ignored_dirs=["node_modules", "NODE_MODULES"])
    result = cfg.effective_ignored_dirs()
    assert "node_modules" in result


def test_effective_max_text_file_bytes_disabled() -> None:
    cfg = Config(text_file_size_limit_enabled=False, max_text_file_mb=5)
    assert cfg.effective_max_text_file_bytes() is None


def test_effective_max_text_file_bytes_enabled() -> None:
    cfg = Config(text_file_size_limit_enabled=True, max_text_file_mb=2)
    result = cfg.effective_max_text_file_bytes()
    assert result == 2 * 1024 * 1024


def test_effective_max_text_file_bytes_minimum_one_mb() -> None:
    cfg = Config(text_file_size_limit_enabled=True, max_text_file_mb=0)
    result = cfg.effective_max_text_file_bytes()
    assert result == 1 * 1024 * 1024


def test_config_import_export_roundtrip(tmp_path: Path) -> None:
    cfg = Config(ui_zoom=0.8, watch_enabled=True, developer_context="ctx")
    export_path = tmp_path / "exported.json"
    Config.export_settings(export_path, cfg)
    imported = Config.import_settings(export_path)
    assert imported.ui_zoom == pytest.approx(0.8)
    assert imported.watch_enabled is True
    assert imported.developer_context == "ctx"


def test_config_export_is_valid_json(tmp_path: Path) -> None:
    cfg = Config()
    export_path = tmp_path / "cfg.json"
    Config.export_settings(export_path, cfg)
    data = json.loads(export_path.read_text(encoding="utf-8"))
    assert "redact_secrets" in data
    assert "ui_zoom" in data


def test_effective_zip_part_bytes_minimum() -> None:
    cfg = Config(zip_part_limit_mb=0)
    assert cfg.effective_zip_part_bytes() == 1 * 1024 * 1024


def test_normalized_diff_export_mode_legacy_alias() -> None:
    cfg = Config(diff_export_mode="changed_since_ref")
    assert cfg.normalized_diff_export_mode() == "git_ref"
