//! User-defined export profiles: the **second** legacy settings file,
//! `~/.project_exporter_profiles.json` (BLUEPRINT §A.3, "Второй файл"), ported from
//! `services/export_profiles.py`. S1 ported only `Config`; this closes that parity gap
//! (decision Q8, 2026-07-25).
//!
//! The file is a single JSON object with two keys — a free-text `description` the
//! application never reads back, and `profiles`, a map of profile key to a **sparse**
//! object of `Config` overrides. A profile is not a `Config`: it names a built-in
//! `base_profile` to start from and overrides a fixed, legacy-defined subset of
//! `Config` fields ([`UserProfile`]'s typed fields below). Fields outside that subset
//! (`extra_ignored_dirs`, `developer_context`, `ui_zoom`, `language`, `last_root`) were
//! deliberately not overridable in legacy and stay that way here — widening the set is
//! a constants-set change, i.e. a separate owner decision.
//!
//! Deviations from legacy, all deliberate and each explained at its definition:
//! [`io::load`] treats a structurally broken file as an error instead of silently
//! reporting "no profiles"; [`apply_custom_profile`] and [`profile_catalog`] take the
//! already-loaded profiles instead of reading the user's home directory themselves;
//! profiles are stored key-ordered rather than in file order.

mod apply;
mod io;

pub use apply::{CatalogEntry, apply_custom_profile, profile_catalog};
pub use io::{LoadedProfiles, ensure_file, load, save};

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// The whole file: `{"description": ..., "profiles": {...}}`.
///
/// `profiles` is a `BTreeMap`, so iteration and serialization are key-ordered and
/// deterministic. Legacy preserved the order the keys appeared in the file (Python
/// dicts are insertion-ordered), which leaked into the order the profile combobox
/// listed custom entries. That ordering is a UI concern, not core's, and determinism
/// is worth more here than reproducing it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UserProfilesFile {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub profiles: BTreeMap<String, UserProfile>,
}

/// One user profile: a base profile plus a sparse set of `Config` overrides.
///
/// Every field is optional and **individually lenient**: a value of the wrong JSON type
/// is dropped and the rest of the profile still loads. That is what legacy's
/// `apply_custom_profile_if_needed` documented ("silently ignore type mismatches so a
/// bad profile entry doesn't abort the export") but never actually did — Python's
/// `setattr` on a dataclass performs no type check, so legacy *stored* the wrong-typed
/// value and only failed later, unpredictably, wherever the field was read. Static
/// typing makes the documented intent the real behavior; the divergence is from
/// legacy's bug, not from its intent.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    /// Combobox label. Not a `Config` override — see [`profile_catalog`].
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub label: Option<String>,
    /// Fallback label when `label` is absent or empty. Not a `Config` override.
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    /// Built-in profile to start from; falls back to `export_profile`, then to `full`.
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub base_profile: Option<String>,

    /// Present in legacy's allowed-override set but explicitly excluded from being
    /// applied: it only ever serves as a fallback source for `base_profile`.
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub export_profile: Option<String>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub safe_export_mode: Option<String>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub text_file_size_limit_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_text_file_mb: Option<u32>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub redact_secrets: Option<bool>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub include_git_patch: Option<bool>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub include_project_in_zip: Option<bool>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub keep_staging_folder: Option<bool>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub zip_part_limit_mb: Option<u32>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub diff_export_mode: Option<String>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub diff_base_ref: Option<String>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub diff_target_ref: Option<String>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_excluded_files: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_excluded_extensions: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub always_include_files: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub always_include_dirs: Option<Vec<String>>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub incremental_export_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub theme: Option<String>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub watch_clipboard_auto_update: Option<bool>,
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub prompt_goals: Option<Vec<String>>,
}

/// Deserializes a present-but-wrong-typed value to `None` instead of failing the whole
/// profile. See [`UserProfile`] for why this granularity is the ported behavior.
fn lenient<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let value = Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).ok())
}

/// The file legacy created on first use, ported verbatim from
/// `export_profiles.py::default_profiles_file_payload` — description text, profile
/// keys, English labels, and every override value unchanged. These two profiles are
/// examples for the user to edit, not built-in profiles: they live in the user's file
/// and can be deleted.
pub fn default_profiles_file() -> UserProfilesFile {
    let codex_safe = UserProfile {
        label: Some("Codex Safe — AI review with strict sharing defaults".to_string()),
        base_profile: Some("ai_review".to_string()),
        safe_export_mode: Some("safe".to_string()),
        text_file_size_limit_enabled: Some(false),
        redact_secrets: Some(true),
        include_git_patch: Some(false),
        include_project_in_zip: Some(true),
        zip_part_limit_mb: Some(512),
        ..UserProfile::default()
    };
    let security_minimal = UserProfile {
        label: Some("Security Minimal — reports-first security handoff".to_string()),
        base_profile: Some("security".to_string()),
        safe_export_mode: Some("safe".to_string()),
        include_project_in_zip: Some(false),
        include_git_patch: Some(false),
        ..UserProfile::default()
    };

    let mut profiles = BTreeMap::new();
    profiles.insert("codex_safe".to_string(), codex_safe);
    profiles.insert("security_minimal".to_string(), security_minimal);

    UserProfilesFile {
        description: "Editable export profile presets. Custom profiles appear in the Profile combobox after app restart or settings reset.".to_string(),
        profiles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_typed_field_is_dropped_without_losing_the_rest_of_the_profile() {
        let profile: UserProfile = serde_json::from_str(
            r#"{"label": "x", "redact_secrets": "yes please", "include_git_patch": true}"#,
        )
        .unwrap();

        assert_eq!(profile.label.as_deref(), Some("x"));
        assert_eq!(profile.redact_secrets, None);
        assert_eq!(profile.include_git_patch, Some(true));
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let profile: UserProfile =
            serde_json::from_str(r#"{"theme": "dark", "not_a_config_field": 7}"#).unwrap();
        assert_eq!(profile.theme.as_deref(), Some("dark"));
    }

    #[test]
    fn absent_fields_serialize_away() {
        let profile = UserProfile {
            theme: Some("dark".to_string()),
            ..UserProfile::default()
        };
        assert_eq!(
            serde_json::to_string(&profile).unwrap(),
            r#"{"theme":"dark"}"#
        );
    }

    #[test]
    fn default_payload_matches_the_legacy_two_example_profiles() {
        let file = default_profiles_file();
        assert_eq!(
            file.profiles.keys().cloned().collect::<Vec<_>>(),
            vec!["codex_safe".to_string(), "security_minimal".to_string()]
        );
        assert_eq!(
            file.profiles["codex_safe"].base_profile.as_deref(),
            Some("ai_review")
        );
        assert_eq!(file.profiles["codex_safe"].zip_part_limit_mb, Some(512));
        assert_eq!(
            file.profiles["security_minimal"].include_project_in_zip,
            Some(false)
        );
    }
}
