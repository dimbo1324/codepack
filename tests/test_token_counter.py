"""Tests for the token counter utility."""

from __future__ import annotations

from project_exporter_desktop.utils.token_counter import (
    context_fit_rows,
    context_summary_line,
    estimate_tokens,
    format_tokens,
)


def test_estimate_tokens_zero() -> None:
    assert estimate_tokens(0) == 1   # clamped to minimum 1


def test_estimate_tokens_small() -> None:
    # 350 bytes / 3.5 = 100 tokens
    assert estimate_tokens(350) == 100


def test_estimate_tokens_large() -> None:
    # 700_000 bytes ≈ 200K tokens
    result = estimate_tokens(700_000)
    assert 190_000 <= result <= 210_000


def test_format_tokens_small() -> None:
    assert format_tokens(500) == "500"


def test_format_tokens_kilo() -> None:
    assert format_tokens(1_500) == "1K"
    assert format_tokens(85_000) == "85K"


def test_format_tokens_mega() -> None:
    result = format_tokens(1_500_000)
    assert "1.5M" in result or "M" in result


def test_context_fit_rows_fits_all() -> None:
    # 1000 bytes → ~285 tokens → fits all models
    rows = context_fit_rows(1000)
    assert all(fits for _, _, _, fits in rows)


def test_context_fit_rows_fits_none() -> None:
    # 5 MB → ~1.5M tokens → fits only Gemini
    rows = context_fit_rows(5 * 1024 * 1024)
    fits_count = sum(1 for _, _, _, fits in rows if fits)
    # Only Gemini (1M) fits; others (128K, 200K) do not
    assert fits_count <= 1


def test_context_fit_rows_sorted_by_limit() -> None:
    rows = context_fit_rows(1000)
    limits = [limit for _, _, limit, _ in rows]
    assert limits == sorted(limits)


def test_context_fit_rows_contains_expected_models() -> None:
    rows = context_fit_rows(1000)
    names = [name for name, _, _, _ in rows]
    assert any("Claude" in n for n in names)
    assert any("GPT" in n for n in names)
    assert any("Gemini" in n for n in names)


def test_context_summary_line_small() -> None:
    line = context_summary_line(350)   # ~100 tokens
    assert "токенов" in line
    assert "✓" in line


def test_context_summary_line_large() -> None:
    # 10 MB → ~3M tokens → nothing fits
    line = context_summary_line(10 * 1024 * 1024)
    assert "токенов" in line
    assert "✗" in line
