//! `.codepack.toml` — per-project configuration, committed alongside the code
//! (BLUEPRINT §B.7, open question Q6, closed by owner decision 2026-07-25 as "done in
//! S10 together with the CLI's configuration reading").
//!
//! The point of this file is that a **team** shares one set of export rules, the same
//! way it shares `.editorconfig` or `.gitignore`. The global settings file is per
//! machine and per user; this one travels with the repository.
//!
//! ## Deliberate choices
//!
//! * **TOML, not JSON.** This file is written by hand. `.exportignore` already
//!   established that per-project rules live in the repository as plain text.
//! * **Every field optional.** The file expresses *overrides*, not a whole `Config`.
//!   A project that only wants `safe_mode = "safe"` writes one line, and every other
//!   setting keeps coming from wherever it came from before.
//! * **Unknown keys are an error, not a shrug.** A misspelled `safe_moed` that is
//!   silently ignored means a team believes it is exporting safely when it is not.
//!   That failure is quiet, permanent and exactly the class this product exists to
//!   prevent, so `deny_unknown_fields` is the right side to err on. The message names
//!   the key, so the fix is obvious.
//! * **Absent is not the same as empty.** No file means "no project-level opinion",
//!   which is the normal case and never an error.

use std::path::{Path, PathBuf};

use codepack_core::config::Config;
use serde::{Deserialize, Serialize};

use crate::error::{CliError, Result};

/// The file name looked for in the project root.
pub(crate) const PROJECT_CONFIG_FILE_NAME: &str = ".codepack.toml";

/// Overrides a project can declare for itself.
///
/// The set is deliberately narrower than `Config`: it carries what a *team* would agree
/// on about exporting this repository, and omits per-user and per-machine settings
/// (`theme`, `ui_zoom`, `language`, `last_root`, watch behaviour). Committing those to
/// a shared repository would mean one developer's window scale overriding everyone
/// else's.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectConfig {
    pub export_profile: Option<String>,
    pub safe_export_mode: Option<String>,
    pub diff_export_mode: Option<String>,
    pub diff_base_ref: Option<String>,
    pub diff_target_ref: Option<String>,
    pub redact_secrets: Option<bool>,
    pub include_git_patch: Option<bool>,
    pub include_project_in_zip: Option<bool>,
    pub keep_staging_folder: Option<bool>,
    pub text_file_size_limit_enabled: Option<bool>,
    pub max_text_file_mb: Option<u32>,
    pub zip_part_limit_mb: Option<u32>,
    pub token_budget: Option<u64>,
    pub history_keep_last_n: Option<u32>,
    pub extra_ignored_dirs: Option<Vec<String>>,
    pub custom_excluded_files: Option<Vec<String>>,
    pub custom_excluded_extensions: Option<Vec<String>>,
    pub always_include_files: Option<Vec<String>>,
    pub always_include_dirs: Option<Vec<String>>,
    pub developer_context: Option<String>,
}

impl ProjectConfig {
    /// Reads `.codepack.toml` from `project_root`. A missing file yields `None`.
    pub(crate) fn load(project_root: &Path) -> Result<Option<(PathBuf, Self)>> {
        let path = project_root.join(PROJECT_CONFIG_FILE_NAME);
        if !path.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path).map_err(|source| CliError::Read {
            path: path.clone(),
            source,
        })?;
        let parsed: Self =
            toml::from_str(&text).map_err(|source| CliError::ProjectConfigSyntax {
                path: path.clone(),
                source,
            })?;
        Ok(Some((path, parsed)))
    }

    /// Applies every field this file actually sets onto `config`, leaving the rest.
    pub(crate) fn apply_to(&self, config: &mut Config) {
        macro_rules! set {
            ($($field:ident),* $(,)?) => {
                $(
                    if let Some(value) = self.$field.clone() {
                        config.$field = value;
                    }
                )*
            };
        }
        set!(
            export_profile,
            safe_export_mode,
            diff_export_mode,
            diff_base_ref,
            diff_target_ref,
            redact_secrets,
            include_git_patch,
            include_project_in_zip,
            keep_staging_folder,
            text_file_size_limit_enabled,
            max_text_file_mb,
            zip_part_limit_mb,
            token_budget,
            history_keep_last_n,
            extra_ignored_dirs,
            custom_excluded_files,
            custom_excluded_extensions,
            always_include_files,
            always_include_dirs,
            developer_context,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with(contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(PROJECT_CONFIG_FILE_NAME), contents).unwrap();
        dir
    }

    #[test]
    fn a_project_without_the_file_has_no_opinion_and_that_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(ProjectConfig::load(dir.path()).unwrap().is_none());
    }

    #[test]
    fn a_single_line_file_overrides_only_that_setting() {
        let dir = project_with("safe_export_mode = \"safe\"\n");
        let (_, project) = ProjectConfig::load(dir.path()).unwrap().unwrap();

        let mut config = Config::default();
        let untouched_profile = config.export_profile.clone();
        project.apply_to(&mut config);

        assert_eq!(config.safe_export_mode, "safe");
        assert_eq!(
            config.export_profile, untouched_profile,
            "a file that says nothing about the profile must not change it"
        );
    }

    #[test]
    fn a_misspelled_key_is_an_error_naming_the_key() {
        // Silently ignoring this would let a team believe it exports safely when it
        // does not — the exact failure this product exists to prevent.
        let dir = project_with("safe_moed = \"safe\"\n");
        let error = ProjectConfig::load(dir.path()).unwrap_err();
        let rendered = error.to_string();
        assert!(
            rendered.contains(PROJECT_CONFIG_FILE_NAME),
            "the message must name the file: {rendered}"
        );
        let source = std::error::Error::source(&error).unwrap().to_string();
        assert!(
            source.contains("safe_moed"),
            "the message must name the offending key: {source}"
        );
    }

    #[test]
    fn broken_toml_is_an_error_rather_than_being_treated_as_absent() {
        let dir = project_with("this is not toml [[[");
        assert!(ProjectConfig::load(dir.path()).is_err());
    }

    #[test]
    fn lists_replace_rather_than_append() {
        // Stated explicitly because "merge or replace" is a real fork in the road for
        // list-valued settings; replace matches how every other config layer behaves.
        let dir = project_with("always_include_files = [\"keep.txt\"]\n");
        let (_, project) = ProjectConfig::load(dir.path()).unwrap().unwrap();

        let mut config = Config {
            always_include_files: vec!["old.txt".to_string()],
            ..Config::default()
        };
        project.apply_to(&mut config);
        assert_eq!(config.always_include_files, vec!["keep.txt".to_string()]);
    }

    #[test]
    fn every_supported_key_round_trips() {
        // Guards against a field being added to the struct but not to `apply_to`'s
        // macro list, which would silently ignore it.
        let full = ProjectConfig {
            export_profile: Some("security".to_string()),
            safe_export_mode: Some("safe".to_string()),
            diff_export_mode: Some("uncommitted".to_string()),
            diff_base_ref: Some("main".to_string()),
            diff_target_ref: Some("feature".to_string()),
            redact_secrets: Some(false),
            include_git_patch: Some(true),
            include_project_in_zip: Some(false),
            keep_staging_folder: Some(true),
            text_file_size_limit_enabled: Some(true),
            max_text_file_mb: Some(9),
            zip_part_limit_mb: Some(64),
            token_budget: Some(1234),
            history_keep_last_n: Some(7),
            extra_ignored_dirs: Some(vec!["vendor".to_string()]),
            custom_excluded_files: Some(vec!["a".to_string()]),
            custom_excluded_extensions: Some(vec!["bin".to_string()]),
            always_include_files: Some(vec!["b".to_string()]),
            always_include_dirs: Some(vec!["docs".to_string()]),
            developer_context: Some("hello".to_string()),
        };

        let mut config = Config::default();
        full.apply_to(&mut config);

        assert_eq!(config.export_profile, "security");
        assert_eq!(config.safe_export_mode, "safe");
        assert_eq!(config.diff_target_ref, "feature");
        assert!(!config.redact_secrets);
        assert_eq!(config.max_text_file_mb, 9);
        assert_eq!(config.token_budget, 1234);
        assert_eq!(config.history_keep_last_n, 7);
        assert_eq!(config.extra_ignored_dirs, vec!["vendor".to_string()]);
        assert_eq!(config.developer_context, "hello");
    }
}
