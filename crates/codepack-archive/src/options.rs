//! [`ArchiveOptions`]: the subset of `Config` this crate's archive planning/building
//! needs.
//!
//! Mirrors `DiffOptions`/`SecurityOptions`/`ScanOptions`: a small, crate-owned view
//! derived from `codepack_core::config::Config`, not a dependency on `Config` itself
//! throughout the crate's internals.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveOptions {
    pub include_project: bool,
    pub part_limit_bytes: u64,
    /// Container the bundle is written as. Defaults to ZIP for any config that does not
    /// say otherwise, which is every config that predates the choice.
    pub format: crate::format::ArchiveFormat,
}

impl From<&codepack_core::config::Config> for ArchiveOptions {
    fn from(config: &codepack_core::config::Config) -> Self {
        Self {
            include_project: config.include_project_in_zip,
            part_limit_bytes: config.effective_zip_part_bytes(),
            format: crate::format::ArchiveFormat::from_config_value(
                config.normalized_archive_format(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_core::config::Config;

    #[test]
    fn from_config_carries_include_project_and_effective_part_bytes() {
        let config = Config {
            include_project_in_zip: false,
            zip_part_limit_mb: 256,
            ..Config::default()
        };
        let options = ArchiveOptions::from(&config);
        assert!(!options.include_project);
        assert_eq!(options.part_limit_bytes, 256 * 1024 * 1024);
    }

    #[test]
    fn a_config_that_never_heard_of_formats_still_asks_for_zip() {
        let options = ArchiveOptions::from(&Config::default());
        assert_eq!(options.format, crate::format::ArchiveFormat::Zip);
    }

    #[test]
    fn an_explicit_format_reaches_the_archiver() {
        let config = Config {
            archive_format: "7z".to_string(),
            ..Config::default()
        };
        assert_eq!(
            ArchiveOptions::from(&config).format,
            crate::format::ArchiveFormat::SevenZip
        );
    }

    #[test]
    fn from_config_default_matches_legacy_512mb_limit() {
        let options = ArchiveOptions::from(&Config::default());
        assert!(options.include_project);
        assert_eq!(options.part_limit_bytes, 512 * 1024 * 1024);
    }
}
