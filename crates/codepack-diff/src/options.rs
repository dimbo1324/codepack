//! [`DiffOptions`]: the subset of `Config` this crate's selection dispatch needs.

/// Mirrors `SecurityOptions`/`ScanOptions` (`codepack-security`/`codepack-scanner`):
/// a small, crate-owned view derived from `codepack_core::config::Config`, not a
/// dependency on `Config` itself throughout the crate's internals.
///
/// `target_ref` was originally left out on purpose: legacy's
/// `services/diff_service.py::_git_selection` accepts a `_target_ref` parameter and
/// never reads it, so `git_ref` mode always diffed `base..HEAD`, and S4 reproduced
/// that limitation rather than silently improving on it.
///
/// Decision Q9 (2026-07-25) makes the field real (🎯). An empty `target_ref` still
/// resolves to `HEAD`, so every configuration written before this change keeps its
/// exact previous behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffOptions {
    pub mode: String,
    pub base_ref: String,
    pub target_ref: String,
}

impl From<&codepack_core::config::Config> for DiffOptions {
    fn from(config: &codepack_core::config::Config) -> Self {
        Self {
            mode: config.normalized_diff_export_mode().to_string(),
            base_ref: config.diff_base_ref.clone(),
            target_ref: config.diff_target_ref.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_core::config::Config;

    #[test]
    fn from_config_uses_normalized_mode() {
        let config = Config {
            diff_export_mode: "changed_since_ref".to_string(),
            ..Config::default()
        };
        let options = DiffOptions::from(&config);
        assert_eq!(options.mode, "git_ref");
    }

    #[test]
    fn from_config_carries_both_base_and_target_ref() {
        let config = Config {
            diff_base_ref: "main".to_string(),
            diff_target_ref: "feature-branch".to_string(),
            ..Config::default()
        };
        let options = DiffOptions::from(&config);
        assert_eq!(options.base_ref, "main");
        // Was asserted absent until decision Q9 (2026-07-25) made the field real.
        assert_eq!(options.target_ref, "feature-branch");
    }

    #[test]
    fn an_unset_target_ref_stays_empty_so_git_ref_mode_still_means_base_to_head() {
        let options = DiffOptions::from(&Config::default());
        assert!(options.target_ref.is_empty());
    }
}
