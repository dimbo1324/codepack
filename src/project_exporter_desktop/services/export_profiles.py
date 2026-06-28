from __future__ import annotations

import json
from dataclasses import replace
from pathlib import Path
from typing import Any

from ..config import Config
from ..constants import EXPORT_PROFILES, USER_EXPORT_PROFILES_FILE

ALLOWED_PROFILE_OVERRIDE_FIELDS = {
    "export_profile",
    "safe_export_mode",
    "text_file_size_limit_enabled",
    "max_text_file_mb",
    "redact_secrets",
    "include_git_patch",
    "include_project_in_zip",
    "keep_staging_folder",
    "zip_part_limit_mb",
    "diff_export_mode",
    "diff_base_ref",
    "diff_target_ref",
    "custom_excluded_files",
    "custom_excluded_extensions",
    "always_include_files",
    "always_include_dirs",
    "incremental_export_enabled",
    "theme",
    "watch_enabled",
    "watch_clipboard_auto_update",
    "prompt_goals",
}


def default_profiles_file_payload() -> dict[str, Any]:
    return {
        "description": "Editable export profile presets. Custom profiles appear in the Profile combobox after app restart or settings reset.",
        "profiles": {
            "codex_safe": {
                "label": "Codex Safe — AI review with strict sharing defaults",
                "base_profile": "ai_review",
                "safe_export_mode": "safe",
                "text_file_size_limit_enabled": False,
                "redact_secrets": True,
                "include_git_patch": False,
                "include_project_in_zip": True,
                "zip_part_limit_mb": 512,
            },
            "security_minimal": {
                "label": "Security Minimal — reports-first security handoff",
                "base_profile": "security",
                "safe_export_mode": "safe",
                "include_project_in_zip": False,
                "include_git_patch": False,
            },
        },
    }


def ensure_user_profiles_file() -> Path:
    if not USER_EXPORT_PROFILES_FILE.exists():
        USER_EXPORT_PROFILES_FILE.write_text(
            json.dumps(default_profiles_file_payload(), ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
    return USER_EXPORT_PROFILES_FILE


def load_user_profiles() -> dict[str, dict[str, Any]]:
    try:
        data = json.loads(USER_EXPORT_PROFILES_FILE.read_text(encoding="utf-8"))
    except Exception:
        return {}
    profiles = data.get("profiles")
    if not isinstance(profiles, dict):
        return {}
    result: dict[str, dict[str, Any]] = {}
    for key, value in profiles.items():
        if not isinstance(key, str) or not key.strip() or not isinstance(value, dict):
            continue
        result[key.strip()] = dict(value)
    return result


def load_profile_catalog() -> dict[str, str]:
    catalog = dict(EXPORT_PROFILES)
    for key, profile in load_user_profiles().items():
        label = (
            profile.get("label")
            or profile.get("description")
            or f"Custom profile based on {profile.get('base_profile', 'full')}"
        )
        catalog[key] = str(label)
    return catalog


def apply_custom_profile_if_needed(profile_key: str, config: Config) -> Config:
    if profile_key in EXPORT_PROFILES:
        config.export_profile = profile_key
        return config

    profile = load_user_profiles().get(profile_key)
    if not profile:
        config.export_profile = "full"
        return config

    updated = replace(config)
    base_profile = str(profile.get("base_profile") or profile.get("export_profile") or "full")
    updated.export_profile = base_profile if base_profile in EXPORT_PROFILES else "full"
    for field, value in profile.items():
        if field not in ALLOWED_PROFILE_OVERRIDE_FIELDS or field == "export_profile":
            continue
        if hasattr(updated, field):
            try:
                setattr(updated, field, value)
            except Exception:
                pass
    return updated