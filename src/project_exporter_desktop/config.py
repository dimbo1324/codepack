from __future__ import annotations

import json
import logging
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

logger = logging.getLogger(__name__)


@dataclass(slots=True)
class Config:
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
    custom_excluded_files: list[str] = field(default_factory=list)
    custom_excluded_extensions: list[str] = field(default_factory=list)
    always_include_files: list[str] = field(default_factory=list)
    always_include_dirs: list[str] = field(default_factory=list)
    incremental_export_enabled: bool = False
    developer_context: str = ""
    theme: str = "system"
    watch_enabled: bool = False
    watch_clipboard_auto_update: bool = False
    ui_zoom: float = 1.0
    prompt_goals: list[str] = field(
        default_factory=lambda: [
            "architecture_review",
            "bug_hunt",
            "write_tests",
        ]
    )

    @classmethod
    def load(cls) -> Config:
        try:
            if SETTINGS_FILE.exists():
                data = json.loads(SETTINGS_FILE.read_text(encoding="utf-8"))
                known = {f.name for f in cls.__dataclass_fields__.values()}
                data = _migrate_legacy_settings({k: v for k, v in data.items() if k in known})
                return cls(**data)
        except Exception as exc:
            logger.warning("Не удалось загрузить настройки: %s", exc)
            return cls()
        return cls()

    def save(self) -> None:
        try:
            SETTINGS_FILE.write_text(
                json.dumps(asdict(self), ensure_ascii=False, indent=2),
                encoding="utf-8",
            )
        except Exception as exc:
            logger.warning("Не удалось сохранить настройки: %s", exc)

    def effective_ignored_dirs(self) -> frozenset[str]:
        extras = {name.strip().casefold() for name in self.extra_ignored_dirs if name.strip()}
        defaults = {name.casefold() for name in IGNORED_DIR_NAMES}
        return frozenset(defaults | extras)

    def normalized_export_profile(self) -> str:
        return (
            self.export_profile
            if self.export_profile in EXPORT_PROFILES
            else DEFAULT_EXPORT_PROFILE
        )

    def normalized_safe_export_mode(self) -> str:
        return self.safe_export_mode if self.safe_export_mode in SAFE_EXPORT_MODES else "safe"

    def normalized_diff_export_mode(self) -> str:
        aliases = {
            "changed_since_ref": "git_ref",
            "between_refs": "git_ref",
        }
        if self.diff_export_mode in DIFF_EXPORT_MODES:
            return self.diff_export_mode
        if self.diff_export_mode in aliases:
            return aliases[self.diff_export_mode]
        if self.incremental_export_enabled:
            return "last_export"
        return "all"

    def normalized_theme(self) -> str:
        return self.theme if self.theme in {"system", "light", "dark"} else "system"

    def normalized_ui_zoom(self) -> float:
        try:
            z = float(self.ui_zoom)
        except (TypeError, ValueError):
            return 1.0
        return max(0.7, min(1.5, z))

    def effective_max_text_file_bytes(self) -> int | None:
        if not self.text_file_size_limit_enabled:
            return None
        return max(1, int(self.max_text_file_mb)) * 1024 * 1024

    def effective_zip_part_bytes(self) -> int:
        return max(1, int(self.zip_part_limit_mb)) * 1024 * 1024

    @staticmethod
    def export_settings(path: Path, config: Config) -> None:
        path.write_text(
            json.dumps(asdict(config), ensure_ascii=False, indent=2),
            encoding="utf-8",
            newline="\n",
        )

    @classmethod
    def import_settings(cls, path: Path) -> Config:
        data = json.loads(path.read_text(encoding="utf-8"))
        known = {f.name for f in cls.__dataclass_fields__.values()}
        migrated = _migrate_legacy_settings({k: v for k, v in data.items() if k in known})
        return cls(**migrated)


def _migrate_legacy_settings(data: dict[str, Any]) -> dict[str, Any]:
    if "text_file_size_limit_enabled" not in data and "max_text_file_mb" in data:
        try:
            data["text_file_size_limit_enabled"] = int(data["max_text_file_mb"]) != 5
        except Exception:
            data["text_file_size_limit_enabled"] = False
    return data
