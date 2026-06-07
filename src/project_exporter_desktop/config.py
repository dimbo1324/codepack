from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from pathlib import Path

from .constants import DEFAULT_EXPORT_PROFILE, EXPORT_PROFILES, IGNORED_DIR_NAMES, SETTINGS_FILE

@dataclass(slots=True)
class Config:
    last_root: str = str(Path.home())
    max_text_file_mb: int = 5
    redact_secrets: bool = True
    keep_staging_folder: bool = False
    include_project_in_zip: bool = True
    extra_ignored_dirs: list[str] = field(default_factory=list)
    export_profile: str = DEFAULT_EXPORT_PROFILE

    @classmethod
    def load(cls) -> Config:
        try:
            if SETTINGS_FILE.exists():
                data = json.loads(SETTINGS_FILE.read_text(encoding="utf-8"))
                # Tolerate older settings files that lack the new fields.
                known = {f.name for f in cls.__dataclass_fields__.values()}
                data = {k: v for k, v in data.items() if k in known}
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
        extras = {name.strip() for name in self.extra_ignored_dirs if name.strip()}
        return IGNORED_DIR_NAMES | extras

    def normalized_export_profile(self) -> str:
        """Return a known export profile; tolerate old or manually edited settings."""
        return self.export_profile if self.export_profile in EXPORT_PROFILES else DEFAULT_EXPORT_PROFILE
