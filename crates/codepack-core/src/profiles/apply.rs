//! Merging user profiles with the built-in ones, ported from
//! `services/export_profiles.py::{load_profile_catalog, apply_custom_profile_if_needed}`.

use super::{UserProfile, UserProfilesFile};
use crate::config::Config;
use crate::config::EXPORT_PROFILES;

/// One entry of the profile picker: a key and the label to show for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub key: String,
    pub label: String,
}

/// Built-in profiles followed by the user's own, exactly as legacy's
/// `load_profile_catalog` composed them (user entries override a built-in of the same
/// key).
///
/// Unlike legacy this takes the already-loaded profiles rather than reading the user's
/// home directory itself: a function that silently touches the filesystem is not
/// testable, and `codepack-core` deliberately leaves path resolution to its caller.
pub fn profile_catalog(profiles: &UserProfilesFile) -> Vec<CatalogEntry> {
    let mut catalog: Vec<CatalogEntry> = EXPORT_PROFILES
        .iter()
        .map(|key| CatalogEntry {
            key: (*key).to_string(),
            label: (*key).to_string(),
        })
        .collect();

    for (key, profile) in &profiles.profiles {
        let label = label_for(profile);
        match catalog.iter_mut().find(|entry| entry.key == *key) {
            Some(existing) => existing.label = label,
            None => catalog.push(CatalogEntry {
                key: key.clone(),
                label,
            }),
        }
    }
    catalog
}

/// Legacy's label precedence: `label`, else `description`, else a generated string
/// naming the base profile.
fn label_for(profile: &UserProfile) -> String {
    let from_label = profile.label.as_deref().filter(|value| !value.is_empty());
    let from_description = profile
        .description
        .as_deref()
        .filter(|value| !value.is_empty());
    match from_label.or(from_description) {
        Some(label) => label.to_string(),
        None => format!(
            "Custom profile based on {}",
            profile.base_profile.as_deref().unwrap_or("full")
        ),
    }
}

/// Resolves `profile_key` against the built-in profiles and the user's own, returning
/// the `Config` the export should actually run with.
///
/// Legacy's exact precedence, reproduced:
/// * a built-in key simply sets `export_profile` and changes nothing else;
/// * an unknown key falls back to `full` rather than failing — a profile the user
///   deleted must not block an export;
/// * a user profile starts from `base_profile`, then `export_profile`, then `full`, and
///   a base naming a profile that no longer exists also falls back to `full`;
/// * `export_profile` is never applied as an override, only consulted as that fallback.
pub fn apply_custom_profile(
    config: &Config,
    profiles: &UserProfilesFile,
    profile_key: &str,
) -> Config {
    let mut updated = config.clone();

    if EXPORT_PROFILES.contains(&profile_key) {
        updated.export_profile = profile_key.to_string();
        return updated;
    }

    let Some(profile) = profiles.profiles.get(profile_key) else {
        updated.export_profile = "full".to_string();
        return updated;
    };

    let base = profile
        .base_profile
        .as_deref()
        .filter(|value| !value.is_empty())
        .or(profile.export_profile.as_deref())
        .filter(|value| !value.is_empty())
        .unwrap_or("full");
    updated.export_profile = if EXPORT_PROFILES.contains(&base) {
        base.to_string()
    } else {
        "full".to_string()
    };

    apply_overrides(&mut updated, profile);
    updated
}

/// Applies every overridable field the profile actually sets. The set mirrors legacy's
/// `ALLOWED_PROFILE_OVERRIDE_FIELDS` minus `export_profile`, which legacy explicitly
/// skips in the same loop.
fn apply_overrides(config: &mut Config, profile: &UserProfile) {
    macro_rules! set {
        ($($field:ident),* $(,)?) => {
            $(
                if let Some(value) = profile.$field.clone() {
                    config.$field = value;
                }
            )*
        };
    }

    set!(
        safe_export_mode,
        text_file_size_limit_enabled,
        max_text_file_mb,
        redact_secrets,
        include_git_patch,
        include_project_in_zip,
        keep_staging_folder,
        zip_part_limit_mb,
        diff_export_mode,
        diff_base_ref,
        diff_target_ref,
        custom_excluded_files,
        custom_excluded_extensions,
        always_include_files,
        always_include_dirs,
        incremental_export_enabled,
        theme,
        watch_enabled,
        watch_clipboard_auto_update,
        prompt_goals,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::default_profiles_file;

    #[test]
    fn a_builtin_key_only_sets_export_profile() {
        let config = Config {
            redact_secrets: false,
            ..Config::default()
        };
        let applied = apply_custom_profile(&config, &UserProfilesFile::default(), "security");

        assert_eq!(applied.export_profile, "security");
        assert!(
            !applied.redact_secrets,
            "a built-in key must not touch any other field"
        );
    }

    #[test]
    fn an_unknown_key_falls_back_to_full_instead_of_failing() {
        let applied =
            apply_custom_profile(&Config::default(), &UserProfilesFile::default(), "deleted");
        assert_eq!(applied.export_profile, "full");
    }

    #[test]
    fn a_user_profile_starts_from_its_base_and_applies_its_overrides() {
        let profiles = default_profiles_file();
        let applied = apply_custom_profile(&Config::default(), &profiles, "codex_safe");

        assert_eq!(applied.export_profile, "ai_review");
        assert_eq!(applied.safe_export_mode, "safe");
        assert!(applied.redact_secrets);
        assert!(!applied.include_git_patch);
        assert_eq!(applied.zip_part_limit_mb, 512);
    }

    #[test]
    fn a_base_profile_naming_something_that_no_longer_exists_falls_back_to_full() {
        let mut profiles = UserProfilesFile::default();
        profiles.profiles.insert(
            "mine".to_string(),
            UserProfile {
                base_profile: Some("a_profile_that_was_removed".to_string()),
                theme: Some("dark".to_string()),
                ..UserProfile::default()
            },
        );

        let applied = apply_custom_profile(&Config::default(), &profiles, "mine");
        assert_eq!(applied.export_profile, "full");
        assert_eq!(applied.theme, "dark", "overrides still apply");
    }

    #[test]
    fn export_profile_serves_only_as_a_base_fallback_never_as_an_override() {
        let mut profiles = UserProfilesFile::default();
        profiles.profiles.insert(
            "mine".to_string(),
            UserProfile {
                export_profile: Some("minimal".to_string()),
                ..UserProfile::default()
            },
        );

        let applied = apply_custom_profile(&Config::default(), &profiles, "mine");
        assert_eq!(
            applied.export_profile, "minimal",
            "used as the base, because base_profile was absent"
        );
    }

    #[test]
    fn the_catalog_lists_builtins_first_then_user_entries() {
        let catalog = profile_catalog(&default_profiles_file());
        let keys: Vec<&str> = catalog.iter().map(|entry| entry.key.as_str()).collect();

        assert_eq!(&keys[..EXPORT_PROFILES.len()], EXPORT_PROFILES);
        assert!(keys.contains(&"codex_safe"));
        assert_eq!(
            catalog
                .iter()
                .find(|entry| entry.key == "codex_safe")
                .map(|entry| entry.label.as_str()),
            Some("Codex Safe — AI review with strict sharing defaults")
        );
    }

    #[test]
    fn a_labelless_profile_gets_the_generated_label() {
        let mut profiles = UserProfilesFile::default();
        profiles.profiles.insert(
            "mine".to_string(),
            UserProfile {
                base_profile: Some("quick".to_string()),
                ..UserProfile::default()
            },
        );

        let catalog = profile_catalog(&profiles);
        assert_eq!(
            catalog
                .iter()
                .find(|entry| entry.key == "mine")
                .map(|entry| entry.label.as_str()),
            Some("Custom profile based on quick")
        );
    }

    #[test]
    fn a_user_entry_reusing_a_builtin_key_relabels_it_without_duplicating_it() {
        let mut profiles = UserProfilesFile::default();
        profiles.profiles.insert(
            "quick".to_string(),
            UserProfile {
                label: Some("My quick".to_string()),
                ..UserProfile::default()
            },
        );

        let catalog = profile_catalog(&profiles);
        assert_eq!(catalog.iter().filter(|e| e.key == "quick").count(), 1);
        assert_eq!(
            catalog
                .iter()
                .find(|entry| entry.key == "quick")
                .map(|entry| entry.label.as_str()),
            Some("My quick")
        );
    }
}
