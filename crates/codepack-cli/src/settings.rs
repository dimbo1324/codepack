//! Turning "what the user asked for" into a single [`Config`].
//!
//! Four sources contribute, and they are applied in this order, each overriding the
//! last:
//!
//! 1. **Built-in defaults** — `Config::default()`.
//! 2. **Global settings** — the user's own file, per machine and per user.
//! 3. **`.codepack.toml`** — the project's committed opinion, shared by the team.
//! 4. **Command-line flags** — this invocation.
//!
//! The order is the conventional one and each step is justified by how far the source
//! travels: a flag applies to one run, a project file to one repository, the global
//! file to one machine. The narrower scope wins.
//!
//! `--preset` sits between (3) and (4): a preset is a named bundle of flags, so it must
//! not override a flag the user typed explicitly on the same command line.

use std::path::Path;

use codepack_core::config::{AiPreset, Config, ai_presets};

use crate::error::{CliError, Result};
use crate::project_config::ProjectConfig;

/// Where each setting ended up coming from, for `--json` consumers and for the human
/// summary. Without this a user cannot tell why an export used `safe` mode.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub(crate) struct ResolutionTrace {
    /// Absolute path of the project config that contributed, if any.
    pub project_config: Option<String>,
    /// Name of the AI preset applied, if any.
    pub preset: Option<String>,
    /// Name of the user-defined export profile applied, if any.
    pub profile: Option<String>,
}

/// The flag values that can override configuration, already parsed.
#[derive(Debug, Clone, Default)]
pub(crate) struct Overrides {
    pub preset: Option<String>,
    pub profile: Option<String>,
    pub safe_mode: Option<String>,
    pub diff: Option<String>,
    pub budget: Option<u64>,
}

/// Resolves the four layers into one [`Config`], reporting what contributed.
pub(crate) fn resolve(
    base: Config,
    project_root: &Path,
    overrides: &Overrides,
    user_profiles: &codepack_core::profiles::UserProfilesFile,
) -> Result<(Config, ResolutionTrace)> {
    let mut config = base;
    let mut trace = ResolutionTrace::default();

    if let Some((path, project)) = ProjectConfig::load(project_root)? {
        project.apply_to(&mut config);
        trace.project_config = Some(path.display().to_string());
    }

    // A profile is resolved before a preset so that an explicit `--preset` still wins:
    // both set overlapping fields, and the preset is the more specific request.
    if let Some(name) = &overrides.profile {
        config = codepack_core::profiles::apply_custom_profile(&config, user_profiles, name);
        trace.profile = Some(name.clone());
    }

    if let Some(name) = &overrides.preset {
        let preset = find_preset(name)?;
        apply_preset(&mut config, preset);
        trace.preset = Some(preset.name.to_string());
    }

    // Individual flags last: the user typed these on this command line, so nothing
    // should be able to override them.
    if let Some(safe_mode) = &overrides.safe_mode {
        config.safe_export_mode = safe_mode.clone();
    }
    if let Some(diff) = &overrides.diff {
        config.diff_export_mode = diff.clone();
    }
    if let Some(budget) = overrides.budget {
        config.token_budget = budget;
    }

    Ok((config, trace))
}

/// Looks a preset up by name, case-insensitively, listing the valid names on failure.
///
/// Rejecting an unknown preset rather than falling back to the default is deliberate:
/// `--preset clade` (a typo) silently producing a default export is precisely the kind
/// of quiet wrong answer a CI pipeline would never notice.
pub(crate) fn find_preset(name: &str) -> Result<&'static AiPreset> {
    let wanted = name.trim().to_lowercase();
    ai_presets()
        .iter()
        .find(|preset| preset.name.to_lowercase() == wanted)
        .ok_or_else(|| {
            let known = ai_presets()
                .iter()
                .map(|preset| preset.name)
                .collect::<Vec<_>>()
                .join(", ");
            CliError::message(format!(
                "unknown preset `{name}`; available presets: {known}"
            ))
        })
}

fn apply_preset(config: &mut Config, preset: &AiPreset) {
    config.export_profile = preset.export_profile.to_string();
    config.safe_export_mode = preset.safe_export_mode.to_string();
    config.redact_secrets = preset.redact_secrets;
    config.include_git_patch = preset.include_git_patch;
    config.diff_export_mode = preset.diff_export_mode.to_string();
    config.text_file_size_limit_enabled = preset.text_file_size_limit_enabled;
    if let Some(max_text_file_mb) = preset.max_text_file_mb {
        config.max_text_file_mb = max_text_file_mb;
    }
}

/// Parses `--budget`, accepting `200000`, `200k` and `1M`.
///
/// BLUEPRINT §B.5's own example is `--budget 200k`, and context windows are quoted in
/// those units everywhere, so requiring a bare integer would make the documented
/// invocation fail.
pub(crate) fn parse_budget(raw: &str) -> std::result::Result<u64, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("budget must not be empty".to_string());
    }

    let (digits, multiplier) = match text.chars().last() {
        Some('k' | 'K') => (&text[..text.len() - 1], 1_000),
        Some('m' | 'M') => (&text[..text.len() - 1], 1_000_000),
        _ => (text, 1),
    };

    let value: u64 = digits.trim().parse().map_err(|_| {
        format!("`{raw}` is not a token budget; use a number, or a number with k/M")
    })?;
    value
        .checked_mul(multiplier)
        .ok_or_else(|| format!("`{raw}` is too large to be a token budget"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_core::profiles::UserProfilesFile;

    fn empty_project() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn budget_accepts_the_units_blueprint_uses() {
        assert_eq!(parse_budget("200000").unwrap(), 200_000);
        assert_eq!(parse_budget("200k").unwrap(), 200_000);
        assert_eq!(parse_budget("200K").unwrap(), 200_000);
        assert_eq!(parse_budget("1M").unwrap(), 1_000_000);
        assert_eq!(parse_budget(" 8k ").unwrap(), 8_000);
    }

    #[test]
    fn budget_rejects_nonsense_instead_of_guessing() {
        assert!(parse_budget("").is_err());
        assert!(parse_budget("lots").is_err());
        assert!(parse_budget("12kb").is_err());
        assert!(parse_budget("-5").is_err());
        assert!(parse_budget("99999999999999999999M").is_err());
    }

    #[test]
    fn a_flag_beats_the_project_file() {
        let dir = empty_project();
        std::fs::write(
            dir.path()
                .join(crate::project_config::PROJECT_CONFIG_FILE_NAME),
            "safe_export_mode = \"full\"\n",
        )
        .unwrap();

        let overrides = Overrides {
            safe_mode: Some("safe".to_string()),
            ..Overrides::default()
        };
        let (config, trace) = resolve(
            Config::default(),
            dir.path(),
            &overrides,
            &UserProfilesFile::default(),
        )
        .unwrap();

        assert_eq!(config.safe_export_mode, "safe");
        assert!(trace.project_config.is_some());
    }

    #[test]
    fn the_project_file_beats_the_global_settings_it_was_given() {
        let dir = empty_project();
        std::fs::write(
            dir.path()
                .join(crate::project_config::PROJECT_CONFIG_FILE_NAME),
            "safe_export_mode = \"safe\"\n",
        )
        .unwrap();

        let global = Config {
            safe_export_mode: "full".to_string(),
            ..Config::default()
        };
        let (config, _) = resolve(
            global,
            dir.path(),
            &Overrides::default(),
            &UserProfilesFile::default(),
        )
        .unwrap();

        assert_eq!(config.safe_export_mode, "safe");
    }

    #[test]
    fn a_preset_beats_a_profile_because_it_is_the_more_specific_request() {
        let dir = empty_project();
        let preset = ai_presets()[0].name.to_string();
        let overrides = Overrides {
            preset: Some(preset.clone()),
            profile: Some("minimal".to_string()),
            ..Overrides::default()
        };

        let (config, trace) = resolve(
            Config::default(),
            dir.path(),
            &overrides,
            &UserProfilesFile::default(),
        )
        .unwrap();

        assert_eq!(trace.preset.as_deref(), Some(preset.as_str()));
        assert_eq!(trace.profile.as_deref(), Some("minimal"));
        assert_eq!(config.export_profile, ai_presets()[0].export_profile);
    }

    #[test]
    fn an_explicit_safe_mode_flag_still_wins_over_a_preset() {
        let dir = empty_project();
        let overrides = Overrides {
            preset: Some(ai_presets()[0].name.to_string()),
            safe_mode: Some("full".to_string()),
            ..Overrides::default()
        };
        let (config, _) = resolve(
            Config::default(),
            dir.path(),
            &overrides,
            &UserProfilesFile::default(),
        )
        .unwrap();
        assert_eq!(config.safe_export_mode, "full");
    }

    #[test]
    fn an_unknown_preset_is_rejected_and_the_message_lists_the_real_ones() {
        let error = find_preset("clade").unwrap_err().to_string();
        assert!(error.contains("clade"));
        for preset in ai_presets() {
            assert!(
                error.contains(preset.name),
                "the message should list `{}`: {error}",
                preset.name
            );
        }
    }

    #[test]
    fn preset_lookup_is_case_insensitive() {
        let name = ai_presets()[0].name;
        assert_eq!(find_preset(&name.to_uppercase()).unwrap().name, name);
    }

    #[test]
    fn nothing_configured_anywhere_yields_the_defaults_untouched() {
        let dir = empty_project();
        let (config, trace) = resolve(
            Config::default(),
            dir.path(),
            &Overrides::default(),
            &UserProfilesFile::default(),
        )
        .unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(trace, ResolutionTrace::default());
    }
}
