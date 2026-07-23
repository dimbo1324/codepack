//! [`DiffOptions`]: the subset of `Config` this crate's selection dispatch needs.

/// Mirrors `SecurityOptions`/`ScanOptions` (`codepack-security`/`codepack-scanner`):
/// a small, crate-owned view derived from `codepack_core::config::Config`, not a
/// dependency on `Config` itself throughout the crate's internals.
///
/// `diff_target_ref` is deliberately **not** carried over. It is a confirmed-dead
/// legacy field: `services/diff_service.py::_git_selection` accepts a `_target_ref`
/// parameter but never reads it — `git_ref` mode always diffs `base..HEAD`, never
/// `base..target`. Replicating that limitation is parity; "fixing" it would be a
/// silent behavior change outside this stage's scope (see `task-checklist.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffOptions {
    pub mode: String,
    pub base_ref: String,
}

impl From<&codepack_core::config::Config> for DiffOptions {
    fn from(config: &codepack_core::config::Config) -> Self {
        Self {
            mode: config.normalized_diff_export_mode().to_string(),
            base_ref: config.diff_base_ref.clone(),
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
    fn from_config_carries_base_ref_but_not_target_ref() {
        let config = Config {
            diff_base_ref: "main".to_string(),
            diff_target_ref: "feature-branch".to_string(),
            ..Config::default()
        };
        let options = DiffOptions::from(&config);
        assert_eq!(options.base_ref, "main");
    }
}
