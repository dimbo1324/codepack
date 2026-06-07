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

def looks_binary(raw: bytes) -> bool:
    if b"\x00" in raw[:8192]:
        return True

    sample = raw[:8192]
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

def read_text_safely(path: Path, max_bytes: int) -> tuple[str | None, str]:
    try:
        raw = path.read_bytes()
    except Exception as exc:
        return None, f"IOError: {exc}"

    if len(raw) == 0:
        return "", "empty"
    if len(raw) > max_bytes:
        return None, f"too-large:{len(raw)}"

    if looks_binary(raw):
        return None, "binary-detected"

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
