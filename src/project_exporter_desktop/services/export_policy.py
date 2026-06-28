# Decides whether a file should be excluded based on the chosen safe-export mode.
# Three modes: 'safe' (strict), 'balanced' (only hard credentials), 'full' (no filtering).

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from ..constants import (
    BALANCED_MODE_EXCLUDED_SUFFIXES,
    HIGH_RISK_FILENAMES,
    SAFE_EXPORT_MODES,
    SAFE_MODE_EXCLUDED_SUFFIXES,
)


@dataclass(frozen=True, slots=True)
class SafetyDecision:
    skip: bool
    reason: str = ""
    severity: str = "medium"


def normalise_mode(mode: str) -> str:
    return mode if mode in SAFE_EXPORT_MODES else "safe"


def is_env_example(name: str) -> bool:
    lowered = name.casefold()
    return (
        lowered.endswith(".example")
        or lowered.endswith(".sample")
        or lowered
        in {
            ".env.example",
            ".env.sample",
        }
    )


def classify_sensitive_file(path: Path) -> SafetyDecision:
    name = path.name.casefold()
    suffix = path.suffix.casefold().lstrip(".")

    if name in HIGH_RISK_FILENAMES or (name.startswith(".env") and not is_env_example(name)):
        return SafetyDecision(
            True, "high-risk credential filename", "critical"
        )  # .env examples are safe to share, real .env files are not
    if "secret" in name or "credential" in name or "private" in name:
        return SafetyDecision(True, "secret-like filename", "high")
    if suffix in SAFE_MODE_EXCLUDED_SUFFIXES:
        return SafetyDecision(True, f"sensitive/binary suffix .{suffix}", "high")
    return SafetyDecision(False)


def should_skip_file_for_safety(relative_path: Path, mode: str) -> SafetyDecision:
    mode = normalise_mode(mode)
    if mode == "full":
        return SafetyDecision(False)

    name = relative_path.name.casefold()
    suffix = relative_path.suffix.casefold().lstrip(".")

    if mode == "balanced":
        if name in HIGH_RISK_FILENAMES or (name.startswith(".env") and not is_env_example(name)):
            return SafetyDecision(True, "high-risk credential filename", "critical")
        if suffix in BALANCED_MODE_EXCLUDED_SUFFIXES:
            return SafetyDecision(True, f"credential/key suffix .{suffix}", "high")
        return SafetyDecision(False)

    return classify_sensitive_file(relative_path)
