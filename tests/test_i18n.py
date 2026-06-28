"""Tests for the i18n service: language switching, key completeness, and string formatting."""

from __future__ import annotations

import pytest

from project_exporter_desktop.i18n import _STRINGS, get_i18n, set_language, t


def _reset_to_ru() -> None:
    get_i18n()._lang = "ru"


@pytest.fixture(autouse=True)
def restore_language():
    original = get_i18n().lang
    yield
    get_i18n()._lang = original


def test_default_language_is_ru() -> None:
    _reset_to_ru()
    assert get_i18n().lang == "ru"


def test_t_returns_string_for_known_key() -> None:
    _reset_to_ru()
    assert isinstance(t("nav.1"), str)
    assert len(t("nav.1")) > 0


def test_t_ru_nav_keys() -> None:
    _reset_to_ru()
    assert "Проект" in t("nav.1")
    assert "Настройки" in t("nav.2")


def test_t_unknown_key_returns_key_itself() -> None:
    _reset_to_ru()
    key = "completely.unknown.key.xyz"
    assert t(key) == key


def test_set_language_to_en() -> None:
    set_language("en")
    assert get_i18n().lang == "en"


def test_t_en_nav_keys() -> None:
    set_language("en")
    assert "Project" in t("nav.1")
    assert "Settings" in t("nav.2")


def test_set_language_back_to_ru() -> None:
    set_language("en")
    set_language("ru")
    assert "Проект" in t("nav.1")


def test_set_language_invalid_ignored() -> None:
    _reset_to_ru()
    set_language("fr")
    assert get_i18n().lang == "ru"


def test_set_same_language_no_signal(monkeypatch) -> None:
    _reset_to_ru()
    called = []
    get_i18n().language_changed.connect(lambda: called.append(1))
    set_language("ru")
    assert called == []


def test_language_changed_signal_fires_on_switch() -> None:
    _reset_to_ru()
    fired = []
    get_i18n().language_changed.connect(lambda: fired.append(1))
    set_language("en")
    assert fired == [1]


def test_all_ru_keys_have_en_counterpart() -> None:
    ru_keys = set(_STRINGS["ru"].keys())
    en_keys = set(_STRINGS["en"].keys())
    missing = ru_keys - en_keys
    assert not missing, f"Keys missing in EN: {missing}"


def test_all_en_keys_have_ru_counterpart() -> None:
    ru_keys = set(_STRINGS["ru"].keys())
    en_keys = set(_STRINGS["en"].keys())
    missing = en_keys - ru_keys
    assert not missing, f"Keys missing in RU: {missing}"


def test_no_empty_strings_in_ru() -> None:
    for key, value in _STRINGS["ru"].items():
        assert value.strip(), f"RU key '{key}' has empty value"


def test_no_empty_strings_in_en() -> None:
    for key, value in _STRINGS["en"].items():
        assert value.strip(), f"EN key '{key}' has empty value"


def test_nav_keys_count() -> None:
    nav_keys = [k for k in _STRINGS["ru"] if k.startswith("nav.")]
    assert len(nav_keys) == 8


def test_diff_hint_keys_present() -> None:
    for mode in ("all", "last_export", "git_ref", "uncommitted"):
        key = f"diff_hint.{mode}"
        assert key in _STRINGS["ru"], f"Missing RU key: {key}"
        assert key in _STRINGS["en"], f"Missing EN key: {key}"


def test_safe_hint_keys_present() -> None:
    for mode in ("safe", "balanced", "full"):
        key = f"safe_hint.{mode}"
        assert key in _STRINGS["ru"], f"Missing RU key: {key}"
        assert key in _STRINGS["en"], f"Missing EN key: {key}"


def test_preset_desc_keys_present() -> None:
    presets = ["Claude Code", "ChatGPT", "Code Review", "Security Audit", "Онбординг"]
    for name in presets:
        key = f"preset.{name}.desc"
        assert key in _STRINGS["ru"], f"Missing RU preset key: {key}"
        assert key in _STRINGS["en"], f"Missing EN preset key: {key}"


def test_page_title_keys_present() -> None:
    page_prefixes = (
        "project",
        "settings",
        "security",
        "preview",
        "run",
        "result",
        "history",
        "analytics",
    )
    for prefix in page_prefixes:
        key = f"{prefix}.page_title"
        assert t(key) != key, f"Key '{key}' not found in string table"


def test_t_fallback_to_ru_when_en_key_missing(monkeypatch) -> None:
    from project_exporter_desktop.i18n import _STRINGS

    original = _STRINGS["en"].get("nav.1")
    try:
        del _STRINGS["en"]["nav.1"]
        set_language("en")
        result = t("nav.1")
        assert "Проект" in result
    finally:
        if original is not None:
            _STRINGS["en"]["nav.1"] = original
        get_i18n()._lang = "ru"


def test_help_html_ru_returns_html() -> None:
    _reset_to_ru()
    html = get_i18n().help_html()
    assert "<html" in html
    assert "Project Exporter" in html


def test_help_html_en_returns_english() -> None:
    set_language("en")
    html = get_i18n().help_html()
    assert "User Manual" in html or "Project Exporter" in html


def test_language_toggle_label_differs() -> None:
    _reset_to_ru()
    ru_label = t("menu.view.language")
    set_language("en")
    en_label = t("menu.view.language")
    assert ru_label != en_label


def test_history_status_keys() -> None:
    _reset_to_ru()
    assert "отменён" in t("history.status_cancelled")
    set_language("en")
    assert "cancel" in t("history.status_cancelled").lower()


def test_snapshot_summary_format() -> None:
    _reset_to_ru()
    s = t("snapshot.summary").format(added=1, modified=2, deleted=3, loc=10)
    assert "1" in s
    set_language("en")
    s = t("snapshot.summary").format(added=1, modified=2, deleted=3, loc=10)
    assert "1" in s


def test_preview_stats_format() -> None:
    for lang in ("ru", "en"):
        set_language(lang)
        result = t("preview.stats").format(inc="5", size="1 MB", exc="3")
        assert "5" in result
        assert "3" in result


def test_result_failed_format() -> None:
    for lang in ("ru", "en"):
        set_language(lang)
        result = t("result.failed").format(log="/tmp/log.txt")
        assert "/tmp/log.txt" in result
