from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from ..constants import (
    BINARY_EXTENSIONS,
    SECRET_PATTERNS,
    TEXT_EXTENSIONS,
    TEXT_FILENAMES_WITHOUT_EXTENSION,
    TRY_ENCODINGS,
)

_BINARY_SAMPLE_BYTES = 8192


def looks_binary(raw: bytes) -> bool:
    if b"\x00" in raw[:_BINARY_SAMPLE_BYTES]:
        return True

    sample = raw[:_BINARY_SAMPLE_BYTES]
    if not sample:
        return False

    control_bytes = sum(1 for b in sample if b < 9 or (13 < b < 32))
    return (control_bytes / len(sample)) > 0.30


def should_consider_text_file(path: Path) -> bool:
    name = path.name.lower()
    suffix = path.suffix.lower().lstrip(".")

    if name in TEXT_FILENAMES_WITHOUT_EXTENSION:
        return True
    if suffix in BINARY_EXTENSIONS:
        return False
    if suffix in TEXT_EXTENSIONS:
        return True

    return False


def read_text_safely(path: Path, max_bytes: int | None = None) -> tuple[str | None, str]:
    """Read a text-like file defensively.

    ``max_bytes=None`` means unlimited. Even then, the function first reads a
    small sample to reject obvious binaries before loading the full file.
    """
    try:
        size = path.stat().st_size
    except Exception as exc:
        return None, f"StatError: {exc}"

    if size == 0:
        return "", "empty"
    if max_bytes is not None and size > max_bytes:
        return None, f"too-large:{size}"

    try:
        with path.open("rb") as file:
            sample = file.read(_BINARY_SAMPLE_BYTES)
            if looks_binary(sample):
                return None, "binary-detected"
            file.seek(0)
            raw = file.read()
    except Exception as exc:
        return None, f"IOError: {exc}"

    for encoding in TRY_ENCODINGS:
        try:
            return raw.decode(encoding), encoding
        except UnicodeDecodeError:
            continue
        except Exception as exc:
            return None, f"DecodeError:{exc}"

    try:
        return raw.decode("latin-1", errors="replace"), "latin-1(replace)"
    except Exception as exc:
        return None, f"DecodeError:{exc}"


def redact_secrets(text: str) -> str:
    redacted = text
    for pattern in SECRET_PATTERNS:

        def repl(match: re.Match[str]) -> str:
            original = match.group(0)
            if "=" in original:
                key = original.split("=", 1)[0]
                return f"{key}=<REDACTED>"
            if ":" in original:
                key = original.split(":", 1)[0]
                return f"{key}: <REDACTED>"
            return "<REDACTED_SECRET>"

        redacted = pattern.sub(repl, redacted)
    return redacted


def format_bytes(size: int) -> str:
    units = ("B", "KB", "MB", "GB", "TB")
    value = float(size)
    for unit in units:
        if value < 1024 or unit == units[-1]:
            if unit == "B":
                return f"{int(value)} {unit}"
            return f"{value:.2f} {unit}"
        value /= 1024
    return f"{size} B"


def safe_read_json(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        return {}
