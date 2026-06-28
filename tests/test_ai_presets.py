"""Tests for AI presets and developer_context config field."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from project_exporter_desktop.config import Config
from project_exporter_desktop.constants import AI_PRESETS, EXPORT_PROFILES, SAFE_EXPORT_MODES


def test_presets_not_empty() -> None:
    assert len(AI_PRESETS) >= 5


def test_each_preset_has_description() -> None:
    for name, preset in AI_PRESETS.items():
        assert "description" in preset, f"Preset '{name}' missing 'description'"
        assert preset["description"], f"Preset '{name}' has empty description"


def test_each_preset_export_profile_is_valid() -> None:
    for name, preset in AI_PRESETS.items():
        if "export_profile" in preset:
            assert preset["export_profile"] in EXPORT_PROFILES, (
                f"Preset '{name}' has unknown export_profile: {preset['export_profile']}"
            )


def test_each_preset_safe_export_mode_is_valid() -> None:
    for name, preset in AI_PRESETS.items():
        if "safe_export_mode" in preset:
            assert preset["safe_export_mode"] in SAFE_EXPORT_MODES, (
                f"Preset '{name}' has unknown safe_export_mode: {preset['safe_export_mode']}"
            )


def test_preset_names_are_unique() -> None:
    assert len(AI_PRESETS) == len(set(AI_PRESETS.keys()))


def test_required_presets_exist() -> None:
    expected = {"Claude Code", "ChatGPT", "Code Review", "Security Audit", "Онбординг"}
    assert expected == set(AI_PRESETS.keys())



def test_developer_context_default_empty() -> None:
    cfg = Config()
    assert cfg.developer_context == ""


def test_developer_context_serialises(tmp_path: Path) -> None:
    cfg = Config()
    cfg.developer_context = "Помоги найти утечки памяти."
    settings_file = tmp_path / "settings.json"
    data = json.dumps({"developer_context": cfg.developer_context})
    settings_file.write_text(data, encoding="utf-8")

    loaded_data = json.loads(settings_file.read_text(encoding="utf-8"))
    assert loaded_data["developer_context"] == "Помоги найти утечки памяти."


def test_config_roundtrip_with_developer_context(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    settings_file = tmp_path / ".project_exporter_desktop.json"
    monkeypatch.setattr(
        "project_exporter_desktop.constants.SETTINGS_FILE", settings_file
    )
    import project_exporter_desktop.config as config_module

    monkeypatch.setattr(config_module, "SETTINGS_FILE", settings_file)

    cfg = Config()
    cfg.developer_context = "Это тестовый контекст."
    cfg.save()

    loaded = Config.load()
    assert loaded.developer_context == "Это тестовый контекст."



def test_claude_code_preset_uses_ai_review() -> None:
    preset = AI_PRESETS["Claude Code"]
    assert preset.get("export_profile") == "ai_review"


def test_security_audit_preset_uses_security_profile() -> None:
    preset = AI_PRESETS["Security Audit"]
    assert preset.get("export_profile") == "security"


def test_chatgpt_preset_has_text_limit() -> None:
    preset = AI_PRESETS["ChatGPT"]
    assert preset.get("text_file_size_limit_enabled") is True


def test_code_review_preset_includes_git_patch() -> None:
    preset = AI_PRESETS["Code Review"]
    assert preset.get("include_git_patch") is True