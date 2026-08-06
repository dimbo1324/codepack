//! Valid-value sets for `Config` string fields, ported verbatim from legacy
//! `constants.py` (keys only — the Russian descriptions are a UI/reports concern,
//! not core's).

pub const EXPORT_PROFILES: &[&str] = &["quick", "full", "ai_review", "security", "minimal"];
pub const DEFAULT_EXPORT_PROFILE: &str = "full";

pub const SAFE_EXPORT_MODES: &[&str] = &["safe", "balanced", "full"];
pub const DEFAULT_SAFE_EXPORT_MODE: &str = "safe";

pub const DIFF_EXPORT_MODES: &[&str] = &["all", "last_export", "git_ref", "uncommitted"];
pub const DEFAULT_DIFF_EXPORT_MODE: &str = "all";

/// Container formats the product can produce. Added 2026-07-30 (owner decision): ZIP
/// stays the default everywhere, so an existing config keeps producing exactly what it
/// produced before. `7z` is fully implemented; `rar` is **declared but not implemented**
/// — RAR compression is patent-encumbered and has no permissively-licensed encoder, so
/// it is listed here so the choice is visible and reserved, and rejected with a clear
/// message wherever it is actually used. See `docs/__arch__/open-questions.md`.
pub const ARCHIVE_FORMATS: &[&str] = &["zip", "7z", "rar"];
pub const DEFAULT_ARCHIVE_FORMAT: &str = "zip";

/// The subset of [`ARCHIVE_FORMATS`] that actually works today. Kept separate rather
/// than filtered at each call site so "listed" and "usable" can never drift apart.
pub const IMPLEMENTED_ARCHIVE_FORMATS: &[&str] = &["zip", "7z"];

/// Local coding agents a bundle can be handed to (stage S13's offline path).
///
/// The ids live here, not in `codepack-ai`, because `Config` has to normalize the
/// stored value and the dependency direction is `ai → core`, never the reverse. The
/// display name and the command belong to `codepack-ai::handoff::AGENTS`, which owns
/// what an agent *is*; a test there asserts the two lists name the same agents, so the
/// split cannot drift into a setting that resolves to nothing.
pub const LOCAL_AI_AGENTS: &[&str] = &["claude-code", "codex"];
pub const DEFAULT_LOCAL_AI_AGENT: &str = "claude-code";

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
