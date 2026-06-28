from __future__ import annotations


_CHARS_PER_TOKEN: float = 3.5

MODEL_CONTEXT_LIMITS: dict[str, int] = {
    "Claude (200K)": 200_000,
    "GPT-4o (128K)": 128_000,
    "GPT-4 Turbo (128K)": 128_000,
    "Gemini 1.5 Pro (1M)": 1_000_000,
}


def estimate_tokens(byte_count: int) -> int:
    return max(1, round(byte_count / _CHARS_PER_TOKEN))


def format_tokens(n: int) -> str:
    if n >= 1_000_000:
        return f"{n / 1_000_000:.1f}M"
    if n >= 1_000:
        return f"{n // 1_000}K"
    return str(n)


def context_fit_rows(byte_count: int) -> list[tuple[str, int, int, bool]]:
    tokens = estimate_tokens(byte_count)
    rows = [
        (name, tokens, limit, tokens <= limit)
        for name, limit in MODEL_CONTEXT_LIMITS.items()
    ]
    return sorted(rows, key=lambda r: r[2])


def context_summary_line(byte_count: int) -> str:
    tokens = estimate_tokens(byte_count)
    tok_str = format_tokens(tokens)
    fits = [name.split(" ")[0] for name, _, limit, ok in context_fit_rows(byte_count) if ok]
    no_fit = [name.split(" ")[0] for name, _, limit, ok in context_fit_rows(byte_count) if not ok]
    parts = [f"~{tok_str} токенов"]
    if fits:
        parts.append("✓ " + ", ".join(fits))
    if no_fit:
        parts.append("✗ " + ", ".join(no_fit))
    return "  │  ".join(parts)