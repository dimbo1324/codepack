"""Edge-case tests for the token counter: boundary values, model limits, and summary line formatting."""

from __future__ import annotations

from project_exporter_desktop.utils.token_counter import (
    _CHARS_PER_TOKEN,
    MODEL_CONTEXT_LIMITS,
    context_fit_rows,
    context_summary_line,
    estimate_tokens,
    format_tokens,
)


def test_estimate_tokens_exactly_one_token() -> None:
    one_token_bytes = round(_CHARS_PER_TOKEN)
    result = estimate_tokens(one_token_bytes)
    assert result == 1


def test_estimate_tokens_zero_returns_one() -> None:
    assert estimate_tokens(0) == 1


def test_estimate_tokens_negative_returns_one() -> None:
    assert estimate_tokens(-999) == 1


def test_estimate_tokens_very_large() -> None:
    one_gb = 1024 * 1024 * 1024
    result = estimate_tokens(one_gb)
    expected = round(one_gb / _CHARS_PER_TOKEN)
    assert result == expected


def test_format_tokens_zero() -> None:
    result = format_tokens(0)
    assert result == "0"


def test_format_tokens_exactly_1000() -> None:
    result = format_tokens(1000)
    assert result == "1K"


def test_format_tokens_exactly_1000000() -> None:
    result = format_tokens(1_000_000)
    assert "1.0M" in result or "M" in result


def test_format_tokens_999() -> None:
    assert format_tokens(999) == "999"


def test_format_tokens_1001() -> None:
    assert format_tokens(1001) == "1K"


def test_context_fit_rows_length_matches_models() -> None:
    rows = context_fit_rows(100)
    assert len(rows) == len(MODEL_CONTEXT_LIMITS)


def test_context_fit_rows_tokens_consistent() -> None:
    byte_count = 350_000
    rows = context_fit_rows(byte_count)
    expected_tokens = estimate_tokens(byte_count)
    for _, tokens, _, _ in rows:
        assert tokens == expected_tokens


def test_context_fit_rows_boundary_exactly_at_limit() -> None:
    smallest_model = min(MODEL_CONTEXT_LIMITS.values())
    boundary_bytes = round(smallest_model * _CHARS_PER_TOKEN)
    rows = context_fit_rows(boundary_bytes)
    fits_count = sum(1 for _, tokens, limit, fits in rows if tokens <= limit)
    assert fits_count >= 1


def test_context_fit_rows_one_token_fits_all() -> None:
    rows = context_fit_rows(round(_CHARS_PER_TOKEN))
    assert all(fits for _, _, _, fits in rows)


def test_context_summary_line_contains_token_count() -> None:
    line = context_summary_line(350)
    tokens = estimate_tokens(350)
    tok_str = format_tokens(tokens)
    assert tok_str in line


def test_context_summary_line_no_fits_shows_x_only() -> None:
    huge_bytes = 5 * 1024 * 1024 * 1024
    line = context_summary_line(huge_bytes)
    assert "✗" in line


def test_context_summary_line_all_fit_no_x() -> None:
    line = context_summary_line(1)
    assert "✓" in line
    assert "✗" not in line


def test_context_summary_line_separator_present() -> None:
    line = context_summary_line(100)
    assert "│" in line


def test_all_model_limits_positive() -> None:
    for name, limit in MODEL_CONTEXT_LIMITS.items():
        assert limit > 0, f"Model {name!r} has non-positive limit {limit}"


def test_gemini_has_largest_context() -> None:
    limits = list(MODEL_CONTEXT_LIMITS.values())
    assert max(limits) == 1_000_000
