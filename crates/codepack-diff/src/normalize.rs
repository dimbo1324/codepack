//! Backslash-separated relative-path normalization, ported verbatim from legacy
//! `diff_service.py::_normalise_rel` (`str(path).replace("/", "\\")`) and
//! `git_diff.py`'s own `.replace("/", "\\")` on every Git-reported path. Every
//! relative path this crate produces — snapshot entries, diff selection entries, and
//! report output — goes through this function so it matches legacy's artifact-facing
//! format regardless of the host OS path separator (`std::path::Path` already yields
//! backslashes on Windows, making this a no-op there; on Linux/macOS it rewrites the
//! forward slashes `Path::strip_prefix`/git2 both produce).

use std::path::Path;

pub(crate) fn to_backslash_path(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_backslash_paths_unchanged() {
        assert_eq!(to_backslash_path(Path::new("src\\main.rs")), "src\\main.rs");
    }

    #[test]
    fn rewrites_forward_slashes_to_backslashes() {
        assert_eq!(to_backslash_path(Path::new("src/main.rs")), "src\\main.rs");
    }

    #[test]
    fn handles_unicode_and_space_containing_segments() {
        assert_eq!(
            to_backslash_path(Path::new("dir with space/файл.rs")),
            "dir with space\\файл.rs"
        );
    }
}
