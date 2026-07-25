//! Global settings, presets and profiles.
//!
//! Presets and profiles are applied **here**, in Rust, rather than by the frontend
//! setting the fields itself. That is deliberate: `codepack_core` already owns what
//! "Claude Code preset" or "minimal profile" means, and the CLI applies it from there.
//! A second implementation in TypeScript would be a second answer to the same question,
//! and the two would eventually disagree — with the GUI quietly exporting something
//! other than the CLI does for the same named preset.

use codepack_core::AppPaths;
use codepack_core::config::{self, Config, ai_presets};
use codepack_core::profiles;

use crate::error::{CommandError, CommandResult};

/// Loads the user's settings file, falling back to defaults when it is absent or
/// unreadable — the same forgiving behaviour `config::load` gives the CLI.
#[tauri::command]
pub fn load_global_settings() -> CommandResult<Config> {
    let paths = AppPaths::resolve()?;
    Ok(config::load(&paths))
}

#[tauri::command]
pub fn save_global_settings(config: Config) -> CommandResult<()> {
    let paths = AppPaths::resolve()?;
    config::save(&paths, &config)?;
    Ok(())
}

/// Applies a built-in AI preset onto `config` and returns the result.
///
/// An unknown name is an error rather than a silent no-op: the frontend only ever sends
/// a name it got from `get_app_info`, so a mismatch means the two sides have drifted,
/// and failing loudly is how that gets noticed.
#[tauri::command]
pub fn apply_preset(config: Config, preset_name: String) -> CommandResult<Config> {
    let wanted = preset_name.trim().to_lowercase();
    let preset = ai_presets()
        .iter()
        .find(|preset| preset.name.to_lowercase() == wanted)
        .ok_or_else(|| {
            let known = ai_presets()
                .iter()
                .map(|preset| preset.name)
                .collect::<Vec<_>>()
                .join(", ");
            CommandError::new(format!(
                "unknown preset `{preset_name}`; available presets: {known}"
            ))
        })?;

    let mut updated = config;
    updated.export_profile = preset.export_profile.to_string();
    updated.safe_export_mode = preset.safe_export_mode.to_string();
    updated.redact_secrets = preset.redact_secrets;
    updated.include_git_patch = preset.include_git_patch;
    updated.diff_export_mode = preset.diff_export_mode.to_string();
    updated.text_file_size_limit_enabled = preset.text_file_size_limit_enabled;
    if let Some(max_text_file_mb) = preset.max_text_file_mb {
        updated.max_text_file_mb = max_text_file_mb;
    }
    Ok(updated)
}

/// Applies a built-in or user-defined export profile.
///
/// Validated before applying, for the reason the CLI documents on its own `--profile`
/// flag: `apply_custom_profile` falls back to `full` for an unknown key, which would
/// *widen* the export while the UI still showed the name the user picked.
#[tauri::command]
pub fn apply_profile(config: Config, profile_name: String) -> CommandResult<Config> {
    let paths = AppPaths::resolve()?;
    let user_profiles = profiles::load(&paths.user_profiles_file())
        .map(|loaded| loaded.file)
        .unwrap_or_default();

    let known = config::EXPORT_PROFILES.contains(&profile_name.as_str())
        || user_profiles.profiles.contains_key(&profile_name);
    if !known {
        let mut names: Vec<String> = config::EXPORT_PROFILES
            .iter()
            .map(|name| (*name).to_string())
            .collect();
        names.extend(user_profiles.profiles.keys().cloned());
        return Err(CommandError::new(format!(
            "unknown profile `{profile_name}`; available profiles: {}",
            names.join(", ")
        )));
    }

    Ok(profiles::apply_custom_profile(
        &config,
        &user_profiles,
        &profile_name,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preset_sets_the_same_fields_the_cli_sets() {
        // The reason presets are applied in Rust: one definition, two front ends.
        let preset = &ai_presets()[0];
        let updated = apply_preset(Config::default(), preset.name.to_string()).unwrap();

        assert_eq!(updated.export_profile, preset.export_profile);
        assert_eq!(updated.safe_export_mode, preset.safe_export_mode);
        assert_eq!(updated.redact_secrets, preset.redact_secrets);
        assert_eq!(updated.include_git_patch, preset.include_git_patch);
        assert_eq!(updated.diff_export_mode, preset.diff_export_mode);
    }

    #[test]
    fn preset_lookup_is_case_insensitive_like_the_cli() {
        let name = ai_presets()[0].name;
        assert!(apply_preset(Config::default(), name.to_uppercase()).is_ok());
    }

    #[test]
    fn an_unknown_preset_is_rejected_and_the_message_lists_the_real_ones() {
        let error = apply_preset(Config::default(), "clade".to_string()).unwrap_err();
        assert!(error.message.contains("clade"));
        for preset in ai_presets() {
            assert!(
                error.message.contains(preset.name),
                "message should list `{}`: {}",
                preset.name,
                error.message
            );
        }
    }

    #[test]
    fn a_preset_leaves_unrelated_settings_alone() {
        // Applying a preset must not silently reset the user's other choices.
        let before = Config {
            developer_context: "refactor the auth module".to_string(),
            keep_staging_folder: true,
            ..Config::default()
        };
        let after = apply_preset(before.clone(), ai_presets()[0].name.to_string()).unwrap();

        assert_eq!(after.developer_context, before.developer_context);
        assert_eq!(after.keep_staging_folder, before.keep_staging_folder);
    }

    #[test]
    fn every_offered_preset_can_actually_be_applied() {
        // Guards the pairing between `get_app_info`'s list and this lookup.
        for preset in super::super::app_info::get_app_info().presets {
            assert!(
                apply_preset(Config::default(), preset.name.clone()).is_ok(),
                "preset `{}` is offered but cannot be applied",
                preset.name
            );
        }
    }
}
