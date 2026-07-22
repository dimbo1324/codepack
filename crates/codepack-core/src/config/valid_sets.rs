//! Valid-value sets for `Config` string fields, ported verbatim from legacy
//! `constants.py` (keys only — the Russian descriptions are a UI/reports concern,
//! not core's).

pub const EXPORT_PROFILES: &[&str] = &["quick", "full", "ai_review", "security", "minimal"];
pub const DEFAULT_EXPORT_PROFILE: &str = "full";

pub const SAFE_EXPORT_MODES: &[&str] = &["safe", "balanced", "full"];
pub const DEFAULT_SAFE_EXPORT_MODE: &str = "safe";

pub const DIFF_EXPORT_MODES: &[&str] = &["all", "last_export", "git_ref", "uncommitted"];
pub const DEFAULT_DIFF_EXPORT_MODE: &str = "all";

pub const THEMES: &[&str] = &["system", "light", "dark"];
pub const DEFAULT_THEME: &str = "system";

/// Not present in legacy (`gui/main_window.py` used a bare `getattr(..., "ru")` with no
/// normalizer). Adding a fallback here is intentionally stricter than legacy behavior —
/// see the S1 final report.
pub const LANGUAGES: &[&str] = &["ru", "en"];
pub const DEFAULT_LANGUAGE: &str = "ru";

/// `diff_export_mode` legacy aliases (`config.py::normalized_diff_export_mode`).
pub(super) fn resolve_diff_export_mode_alias(value: &str) -> &str {
    match value {
        "changed_since_ref" | "between_refs" => "git_ref",
        other => other,
    }
}
