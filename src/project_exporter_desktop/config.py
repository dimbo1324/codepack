from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from .constants import (
    DEFAULT_EXPORT_PROFILE,
    DIFF_EXPORT_MODES,
    EXPORT_PROFILES,
    IGNORED_DIR_NAMES,
    MAX_ARCHIVE_PART_MB,
    SAFE_EXPORT_MODES,
    SETTINGS_FILE,
)


@dataclass(slots=True)
class Config:
    """Persisted user configuration for the desktop exporter.

    The defaults intentionally favour safe sharing: no text-size limit, secret
    redaction enabled, Safe Export mode enabled, and 512 MB archive parts.
    """

    last_root: str = str(Path.home())
    text_file_size_limit_enabled: bool = False
    max_text_file_mb: int = 5
    redact_secrets: bool = True
    keep_staging_folder: bool = False
    include_project_in_zip: bool = True
    extra_ignored_dirs: list[str] = field(default_factory=list)
    export_profile: str = DEFAULT_EXPORT_PROFILE
    safe_export_mode: str = "safe"
    zip_part_limit_mb: int = MAX_ARCHIVE_PART_MB
    diff_export_mode: str = "all"
    diff_base_ref: str = "HEAD"
    diff_target_ref: str = ""
    include_git_patch: bool = False

    @classmethod
    def load(cls) -> Config:
        try:
            if SETTINGS_FILE.exists():
                data = json.loads(SETTINGS_FILE.read_text(encoding="utf-8"))
                known = {f.name for f in cls.__dataclass_fields__.values()}
                data = _migrate_legacy_settings({k: v for k, v in data.items() if k in known})
                return cls(**data)
        except Exception:
            return cls()
        return cls()

    def save(self) -> None:
        try:
            SETTINGS_FILE.write_text(
                json.dumps(asdict(self), ensure_ascii=False, indent=2),
                encoding="utf-8",
            )
        except Exception:
            pass

    def effective_ignored_dirs(self) -> frozenset[str]:
        """Defaults are always present; user values are additive only."""
        extras = {name.strip().casefold() for name in self.extra_ignored_dirs if name.strip()}
        defaults = {name.casefold() for name in IGNORED_DIR_NAMES}
        return frozenset(defaults | extras)

    def normalized_export_profile(self) -> str:
        return self.export_profile if self.export_profile in EXPORT_PROFILES else DEFAULT_EXPORT_PROFILE

    def normalized_safe_export_mode(self) -> str:
        return self.safe_export_mode if self.safe_export_mode in SAFE_EXPORT_MODES else "safe"

    def normalized_diff_export_mode(self) -> str:
        return self.diff_export_mode if self.diff_export_mode in DIFF_EXPORT_MODES else "all"

    def effective_max_text_file_bytes(self) -> int | None:
        if not self.text_file_size_limit_enabled:
            return None
        return max(1, int(self.max_text_file_mb)) * 1024 * 1024

    def effective_zip_part_bytes(self) -> int:
        return max(1, int(self.zip_part_limit_mb)) * 1024 * 1024


# -- Legacy settings ---------------------------------------------------------


def _migrate_legacy_settings(data: dict[str, Any]) -> dict[str, Any]:
    """Tolerate settings files created by older app versions.

    Older versions always had a text-file size limit. Version 4 defaults to no
    limit, but if the old settings file contains a non-default value, we preserve
    the user's likely intention by enabling the limit.
    """
    if "text_file_size_limit_enabled" not in data and "max_text_file_mb" in data:
        try:
            data["text_file_size_limit_enabled"] = int(data["max_text_file_mb"]) != 5
        except Exception:
            data["text_file_size_limit_enabled"] = False
    return data
