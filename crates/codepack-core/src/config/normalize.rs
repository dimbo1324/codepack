//! `normalized_*`/`effective_*` read accessors, ported from `config.py`. These never
//! mutate `Config` — an invalid raw value stays stored as-is; only the accessor falls
//! back to a safe default, matching the legacy `Config` methods of the same name.
//! Two accessors intentionally deviate — see their own doc comments:
//! `normalized_language` (no legacy normalizer existed at all) and
//! `normalized_ui_zoom` (legacy's NaN/infinity fallback was a comparison accident,
//! not a real behavior worth reproducing).

use super::Config;
use super::valid_sets::{
    DEFAULT_DIFF_EXPORT_MODE, DEFAULT_EXPORT_PROFILE, DEFAULT_LANGUAGE, DEFAULT_SAFE_EXPORT_MODE,
    DEFAULT_THEME, DIFF_EXPORT_MODES, EXPORT_PROFILES, LANGUAGES, SAFE_EXPORT_MODES, THEMES,
    resolve_diff_export_mode_alias,
};

pub const UI_ZOOM_MIN: f64 = 0.7;
pub const UI_ZOOM_MAX: f64 = 1.5;
pub const DEFAULT_UI_ZOOM: f64 = 1.0;

const BYTES_PER_MB: u64 = 1024 * 1024;

impl Config {
    pub fn normalized_export_profile(&self) -> &str {
        if EXPORT_PROFILES.contains(&self.export_profile.as_str()) {
            &self.export_profile
        } else {
            DEFAULT_EXPORT_PROFILE
        }
    }

    pub fn normalized_safe_export_mode(&self) -> &str {
        if SAFE_EXPORT_MODES.contains(&self.safe_export_mode.as_str()) {
            &self.safe_export_mode
        } else {
            DEFAULT_SAFE_EXPORT_MODE
        }
    }

    /// Depends on `incremental_export_enabled` — mirrors `config.py`'s
    /// `normalized_diff_export_mode`, which is the same cross-field coupling.
    pub fn normalized_diff_export_mode(&self) -> &str {
        let resolved = resolve_diff_export_mode_alias(&self.diff_export_mode);
        if DIFF_EXPORT_MODES.contains(&resolved) {
            return resolved;
        }
        if self.incremental_export_enabled {
            "last_export"
        } else {
            DEFAULT_DIFF_EXPORT_MODE
        }
    }

    pub fn normalized_theme(&self) -> &str {
        if THEMES.contains(&self.theme.as_str()) {
            &self.theme
        } else {
            DEFAULT_THEME
        }
    }

    /// Not present in legacy; see `valid_sets::LANGUAGES`.
    pub fn normalized_language(&self) -> &str {
        if LANGUAGES.contains(&self.language.as_str()) {
            &self.language
        } else {
            DEFAULT_LANGUAGE
        }
    }

    /// Intentional deviation from legacy for non-finite input. Legacy's
    /// `max(0.7, min(1.5, z))` only falls back to `1.0` when `float(z)` itself raises;
    /// for a `z` that is already a float, Python's `min`/`max` are order-dependent
    /// against NaN and silently produce `1.5` for `nan`/`+inf` and `0.7` for `-inf` —
    /// an accidental comparison quirk, not a deliberate default. Replicating that
    /// quirk would port a bug, not a behavior, so every non-finite value here falls
    /// back to `DEFAULT_UI_ZOOM` instead. Unreachable through JSON load/import
    /// (`serde_json` rejects `NaN`/`Infinity` tokens); only reachable if another crate
    /// constructs a `Config` with a non-finite `ui_zoom` directly.
    pub fn normalized_ui_zoom(&self) -> f64 {
        if self.ui_zoom.is_finite() {
            self.ui_zoom.clamp(UI_ZOOM_MIN, UI_ZOOM_MAX)
        } else {
            DEFAULT_UI_ZOOM
        }
    }

    pub fn effective_max_text_file_bytes(&self) -> Option<u64> {
        if !self.text_file_size_limit_enabled {
            return None;
        }
        Some(u64::from(self.max_text_file_mb.max(1)) * BYTES_PER_MB)
    }

    pub fn effective_zip_part_bytes(&self) -> u64 {
        u64::from(self.zip_part_limit_mb.max(1)) * BYTES_PER_MB
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config::default()
    }

    #[test]
    fn export_profile_passthrough_for_valid_value() {
        let mut cfg = config();
        cfg.export_profile = "security".to_string();
        assert_eq!(cfg.normalized_export_profile(), "security");
    }

    #[test]
    fn export_profile_falls_back_for_invalid_value() {
        let mut cfg = config();
        cfg.export_profile = "bogus".to_string();
        assert_eq!(cfg.normalized_export_profile(), DEFAULT_EXPORT_PROFILE);
    }

    #[test]
    fn safe_export_mode_passthrough_for_valid_value() {
        let mut cfg = config();
        cfg.safe_export_mode = "balanced".to_string();
        assert_eq!(cfg.normalized_safe_export_mode(), "balanced");
    }

    #[test]
    fn safe_export_mode_falls_back_for_invalid_value() {
        let mut cfg = config();
        cfg.safe_export_mode = "reckless".to_string();
        assert_eq!(cfg.normalized_safe_export_mode(), DEFAULT_SAFE_EXPORT_MODE);
    }

    #[test]
    fn diff_export_mode_passthrough_for_valid_value() {
        let mut cfg = config();
        cfg.diff_export_mode = "uncommitted".to_string();
        assert_eq!(cfg.normalized_diff_export_mode(), "uncommitted");
    }

    #[test]
    fn diff_export_mode_resolves_changed_since_ref_alias() {
        let mut cfg = config();
        cfg.diff_export_mode = "changed_since_ref".to_string();
        assert_eq!(cfg.normalized_diff_export_mode(), "git_ref");
    }

    #[test]
    fn diff_export_mode_resolves_between_refs_alias() {
        let mut cfg = config();
        cfg.diff_export_mode = "between_refs".to_string();
        assert_eq!(cfg.normalized_diff_export_mode(), "git_ref");
    }

    #[test]
    fn diff_export_mode_falls_back_to_last_export_when_incremental_enabled() {
        let mut cfg = config();
        cfg.diff_export_mode = "not_a_mode".to_string();
        cfg.incremental_export_enabled = true;
        assert_eq!(cfg.normalized_diff_export_mode(), "last_export");
    }

    #[test]
    fn diff_export_mode_falls_back_to_all_when_incremental_disabled() {
        let mut cfg = config();
        cfg.diff_export_mode = "not_a_mode".to_string();
        cfg.incremental_export_enabled = false;
        assert_eq!(cfg.normalized_diff_export_mode(), DEFAULT_DIFF_EXPORT_MODE);
    }

    #[test]
    fn theme_passthrough_for_valid_value() {
        let mut cfg = config();
        cfg.theme = "dark".to_string();
        assert_eq!(cfg.normalized_theme(), "dark");
    }

    #[test]
    fn theme_falls_back_for_invalid_value() {
        let mut cfg = config();
        cfg.theme = "solarized".to_string();
        assert_eq!(cfg.normalized_theme(), DEFAULT_THEME);
    }

    #[test]
    fn language_passthrough_for_valid_value() {
        let mut cfg = config();
        cfg.language = "en".to_string();
        assert_eq!(cfg.normalized_language(), "en");
    }

    #[test]
    fn language_falls_back_for_invalid_value() {
        let mut cfg = config();
        cfg.language = "fr".to_string();
        assert_eq!(cfg.normalized_language(), DEFAULT_LANGUAGE);
    }

    #[test]
    fn ui_zoom_passthrough_within_range() {
        let mut cfg = config();
        cfg.ui_zoom = 1.2;
        assert_eq!(cfg.normalized_ui_zoom(), 1.2);
    }

    #[test]
    fn ui_zoom_clamps_above_max() {
        let mut cfg = config();
        cfg.ui_zoom = 5.0;
        assert_eq!(cfg.normalized_ui_zoom(), UI_ZOOM_MAX);
    }

    #[test]
    fn ui_zoom_clamps_below_min() {
        let mut cfg = config();
        cfg.ui_zoom = 0.1;
        assert_eq!(cfg.normalized_ui_zoom(), UI_ZOOM_MIN);
    }

    #[test]
    fn ui_zoom_falls_back_on_non_finite_value() {
        let mut cfg = config();
        cfg.ui_zoom = f64::NAN;
        assert_eq!(cfg.normalized_ui_zoom(), DEFAULT_UI_ZOOM);

        cfg.ui_zoom = f64::INFINITY;
        assert_eq!(cfg.normalized_ui_zoom(), DEFAULT_UI_ZOOM);
    }

    #[test]
    fn effective_max_text_file_bytes_none_when_limit_disabled() {
        let mut cfg = config();
        cfg.text_file_size_limit_enabled = false;
        cfg.max_text_file_mb = 10;
        assert_eq!(cfg.effective_max_text_file_bytes(), None);
    }

    #[test]
    fn effective_max_text_file_bytes_converts_mb_to_bytes_when_enabled() {
        let mut cfg = config();
        cfg.text_file_size_limit_enabled = true;
        cfg.max_text_file_mb = 5;
        assert_eq!(cfg.effective_max_text_file_bytes(), Some(5 * 1024 * 1024));
    }

    #[test]
    fn effective_max_text_file_bytes_clamps_zero_to_one_mb() {
        let mut cfg = config();
        cfg.text_file_size_limit_enabled = true;
        cfg.max_text_file_mb = 0;
        assert_eq!(cfg.effective_max_text_file_bytes(), Some(1024 * 1024));
    }

    #[test]
    fn effective_zip_part_bytes_converts_mb_to_bytes() {
        let mut cfg = config();
        cfg.zip_part_limit_mb = 512;
        assert_eq!(cfg.effective_zip_part_bytes(), 512 * 1024 * 1024);
    }

    #[test]
    fn effective_zip_part_bytes_clamps_zero_to_one_mb() {
        let mut cfg = config();
        cfg.zip_part_limit_mb = 0;
        assert_eq!(cfg.effective_zip_part_bytes(), 1024 * 1024);
    }
}
