//! What the app is and what it offers — read once at startup.

use codepack_core::config::{EXPORT_PROFILES, PROJECT_CONFIG_FILE_NAME, ai_presets};

use crate::dto::{AppInfo, PresetInfo, ProfileInfo};

/// Shown verbatim in the UI. Sent from the backend, not hardcoded in the page, so it
/// describes what this binary does rather than what a page author assumed — invariant I1
/// is a property of the code, and this is the code saying so.
const NETWORK_ACCESS_STATEMENT: &str =
    "This application never accesses the network. All analysis is local.";

/// Human labels for the five built-in export profiles.
///
/// The `key` is what `Config::export_profile` stores; the `label` is what a combo box
/// shows. Kept beside the key list rather than in the frontend so a profile added to the
/// core cannot silently appear in the UI without a name.
fn label_for(profile: &str) -> &'static str {
    match profile {
        "quick" => "Quick — overview, entry points, key files",
        "full" => "Full — every report and the AI context bundle",
        "ai_review" => "AI review — everything an assistant needs to change code safely",
        "security" => "Security — config, dependencies, Git, code quality, secrets",
        "minimal" => "Minimal — a compact bundle that is easy to hand over",
        _ => "Custom profile",
    }
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        presets: ai_presets()
            .iter()
            .map(|preset| PresetInfo {
                name: preset.name.to_string(),
                description: preset.description.to_string(),
                export_profile: preset.export_profile.to_string(),
                safe_mode: preset.safe_export_mode.to_string(),
            })
            .collect(),
        profiles: EXPORT_PROFILES
            .iter()
            .map(|key| ProfileInfo {
                key: (*key).to_string(),
                label: label_for(key).to_string(),
            })
            .collect(),
        project_config_file_name: PROJECT_CONFIG_FILE_NAME.to_string(),
        network_access: NETWORK_ACCESS_STATEMENT.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_preset_is_offered_to_the_user() {
        let info = get_app_info();
        assert_eq!(info.presets.len(), ai_presets().len());
        for preset in ai_presets() {
            assert!(
                info.presets.iter().any(|shown| shown.name == preset.name),
                "preset {} is missing from the UI list",
                preset.name
            );
        }
    }

    #[test]
    fn every_export_profile_has_a_human_label() {
        // A profile added to the core must not show up as a bare key in a combo box.
        let info = get_app_info();
        assert_eq!(info.profiles.len(), EXPORT_PROFILES.len());
        for profile in &info.profiles {
            assert!(!profile.label.is_empty());
            assert_ne!(
                profile.label, "Custom profile",
                "profile `{}` has no label of its own",
                profile.key
            );
        }
    }

    #[test]
    fn the_version_is_the_real_crate_version() {
        assert_eq!(get_app_info().version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn the_network_statement_is_present_for_the_ui_to_show() {
        // Invariant I1 is a selling point of this product, and the UI states it.
        let info = get_app_info();
        assert!(info.network_access.to_lowercase().contains("never"));
        assert!(info.network_access.to_lowercase().contains("network"));
    }

    #[test]
    fn the_project_config_file_name_matches_what_the_cli_reads() {
        // The GUI must not tell the user about a file the CLI does not look for.
        assert_eq!(get_app_info().project_config_file_name, ".codepack.toml");
    }
}
