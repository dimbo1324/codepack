//! [`ArchiveFormat`]: which container an archive is written as.
//!
//! Added 2026-07-30 (owner decision). **ZIP is the default everywhere**, so a config
//! that predates this choice keeps producing byte-for-byte what it produced before —
//! the split-set contract (invariant I5) is a ZIP contract and stays one unless a user
//! explicitly asks for something else.
//!
//! `rar` is deliberately present but not implemented. RAR's compression algorithm is
//! proprietary and there is no permissively-licensed encoder to depend on, so shipping
//! it would mean either a licence problem or shelling out to a binary the user is not
//! guaranteed to have. Listing it and refusing it with a reason is more honest than
//! omitting it and leaving the user to wonder — and than silently writing a ZIP with a
//! `.rar` name, which is what a "graceful fallback" would amount to.

use std::path::Path;

use crate::error::{ArchiveError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchiveFormat {
    #[default]
    Zip,
    SevenZip,
    /// Declared, reserved, and refused at the point of use. See the module docs.
    Rar,
}

impl ArchiveFormat {
    /// Parses a `Config::archive_format` value. Anything unrecognised is [`Self::Zip`],
    /// matching how every other string field in `Config` normalizes.
    #[must_use]
    pub fn from_config_value(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "7z" | "sevenzip" | "seven_zip" => Self::SevenZip,
            "rar" => Self::Rar,
            _ => Self::Zip,
        }
    }

    /// Guesses from a file name, for the places where the user names the file rather
    /// than the format (`codepack sanitize --archive out.7z`). `None` when the
    /// extension says nothing, so the caller can fall back to its own default rather
    /// than have one chosen for it.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_string_lossy().to_lowercase();
        match extension.as_str() {
            "zip" => Some(Self::Zip),
            "7z" => Some(Self::SevenZip),
            "rar" => Some(Self::Rar),
            _ => None,
        }
    }

    /// The value as it appears in `Config`, in `--json` output and on the command line.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::SevenZip => "7z",
            Self::Rar => "rar",
        }
    }

    /// File extension, without the dot. Same string as [`Self::as_str`] today, kept as
    /// its own method so a format whose name and extension differ does not have to
    /// change every call site.
    #[must_use]
    pub fn extension(self) -> &'static str {
        self.as_str()
    }

    /// Whether this format can actually be written. `false` only for [`Self::Rar`].
    #[must_use]
    pub fn is_implemented(self) -> bool {
        !matches!(self, Self::Rar)
    }

    /// The check every writer runs before touching the filesystem, so an unimplemented
    /// format fails before a partial file exists rather than after.
    pub fn ensure_implemented(self) -> Result<()> {
        if self.is_implemented() {
            Ok(())
        } else {
            Err(ArchiveError::FormatNotImplemented {
                format: self.as_str(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_values_round_trip() {
        for format in [
            ArchiveFormat::Zip,
            ArchiveFormat::SevenZip,
            ArchiveFormat::Rar,
        ] {
            assert_eq!(ArchiveFormat::from_config_value(format.as_str()), format);
        }
    }

    #[test]
    fn an_unrecognised_config_value_is_zip_not_a_failure() {
        // Same rule as every other string field in `Config`: normalize, do not reject.
        assert_eq!(
            ArchiveFormat::from_config_value("tar.gz"),
            ArchiveFormat::Zip
        );
        assert_eq!(ArchiveFormat::from_config_value(""), ArchiveFormat::Zip);
        assert_eq!(ArchiveFormat::default(), ArchiveFormat::Zip);
    }

    #[test]
    fn parsing_is_case_and_whitespace_insensitive() {
        assert_eq!(
            ArchiveFormat::from_config_value(" 7Z "),
            ArchiveFormat::SevenZip
        );
        assert_eq!(ArchiveFormat::from_config_value("RAR"), ArchiveFormat::Rar);
    }

    #[test]
    fn a_file_name_decides_the_format_when_it_says_anything() {
        assert_eq!(
            ArchiveFormat::from_path(Path::new("out.7z")),
            Some(ArchiveFormat::SevenZip)
        );
        assert_eq!(
            ArchiveFormat::from_path(Path::new("out.ZIP")),
            Some(ArchiveFormat::Zip)
        );
        assert_eq!(
            ArchiveFormat::from_path(Path::new("out.rar")),
            Some(ArchiveFormat::Rar)
        );
        // Says nothing — the caller keeps its own default rather than being given one.
        assert_eq!(ArchiveFormat::from_path(Path::new("out")), None);
        assert_eq!(ArchiveFormat::from_path(Path::new("out.tar")), None);
    }

    #[test]
    fn rar_is_listed_but_refused_with_a_reason() {
        assert!(!ArchiveFormat::Rar.is_implemented());
        let error = ArchiveFormat::Rar.ensure_implemented().unwrap_err();
        let message = error.to_string();
        assert!(message.contains("rar"), "{message}");
        assert!(
            message.contains("zip") && message.contains("7z"),
            "the message must name what the user *can* pick: {message}"
        );
    }

    #[test]
    fn the_two_working_formats_are_implemented() {
        assert!(ArchiveFormat::Zip.is_implemented());
        assert!(ArchiveFormat::SevenZip.is_implemented());
        ArchiveFormat::Zip.ensure_implemented().unwrap();
        ArchiveFormat::SevenZip.ensure_implemented().unwrap();
    }

    /// The two lists in `codepack-core` must agree with this type, or the UI would
    /// offer a format the archiver rejects, or hide one it supports.
    #[test]
    fn the_core_valid_sets_match_this_type() {
        use codepack_core::config::{ARCHIVE_FORMATS, IMPLEMENTED_ARCHIVE_FORMATS};

        for name in ARCHIVE_FORMATS {
            assert_eq!(ArchiveFormat::from_config_value(name).as_str(), *name);
        }
        for name in ARCHIVE_FORMATS {
            let format = ArchiveFormat::from_config_value(name);
            assert_eq!(
                format.is_implemented(),
                IMPLEMENTED_ARCHIVE_FORMATS.contains(name),
                "{name} disagrees between the valid-set list and the archiver"
            );
        }
    }
}
