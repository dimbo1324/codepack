//! Helpers for `PlannedFile.relative_path` (`codepack-scanner`'s backslash-joined
//! display string — see `codepack_scanner::plan::build::display_backslash`), which is
//! **not** an OS `Path`: on a non-Windows CI runner, `Path::new("src\\main.py")` is a
//! single filename component (Unix has no `\` separator), so calling
//! `.file_name()`/`.extension()` on it directly would silently misclassify multi-level
//! paths. Every helper here treats `\` as the one true separator, regardless of host
//! OS, and [`to_native_path`] is the only bridge back to `std::path::Path` (used when a
//! report needs to actually open the file on disk under `staging_root`, or call into
//! `codepack_scanner`'s `Path`-based classifiers).

use std::path::{Path, PathBuf};

/// The final path segment, i.e. everything after the last `\` (or the whole string if
/// there is none).
pub(crate) fn file_name_of(relative_path: &str) -> &str {
    match relative_path.rfind('\\') {
        Some(index) => &relative_path[index + 1..],
        None => relative_path,
    }
}

/// Number of non-empty `\`-separated segments (legacy `path_depth`).
pub(crate) fn path_depth(relative_path: &str) -> usize {
    relative_path
        .split('\\')
        .filter(|part| !part.is_empty())
        .count()
}

/// Lowercased suffix without the leading dot, or empty when the name has no extension
/// (matching Python's `Path.suffix`: a name that starts with `.` and has no other dot,
/// e.g. `.env`, has an empty suffix).
fn suffix_of(file_name: &str) -> String {
    match file_name.rfind('.') {
        Some(0) | None => String::new(),
        Some(index) => file_name[index + 1..].to_lowercase(),
    }
}

/// Legacy `extension_key`: `dockerfile`/`dockerfile.*` collapse to `"dockerfile"`;
/// otherwise the lowercased suffix, or `"[no extension]"` when there is none.
pub(crate) fn extension_key(relative_path: &str) -> String {
    let name = file_name_of(relative_path).to_lowercase();
    if name == "dockerfile" || name.starts_with("dockerfile.") {
        return "dockerfile".to_string();
    }
    let suffix = suffix_of(&name);
    if suffix.is_empty() {
        "[no extension]".to_string()
    } else {
        suffix
    }
}

/// Rebuilds a native, component-correct [`PathBuf`] from a `\`-joined relative path,
/// so it can be joined onto `staging_root` or passed to a `Path`-based API safely on
/// every host OS.
pub(crate) fn to_native_path(relative_path: &str) -> PathBuf {
    let mut buf = PathBuf::new();
    for part in relative_path.split('\\').filter(|part| !part.is_empty()) {
        buf.push(part);
    }
    buf
}

/// [`to_native_path`], joined onto `root`.
pub(crate) fn resolve(root: &Path, relative_path: &str) -> PathBuf {
    root.join(to_native_path(relative_path))
}

/// Legacy `re.search(r"(?i)(^|[._/-])(test|spec)([._/-]|$)", str(relative_path))`,
/// reimplemented per `\`-separated segment rather than against the whole joined
/// string: legacy's character class (`[._/-]`) never includes `\`, so applying that
/// same regex verbatim to our always-backslash-joined `relative_path` would silently
/// stop matching at every segment boundary (a latent legacy inconsistency between its
/// Windows path rendering and its own Unix-flavored regex, not something to reproduce
/// — see the crate-scope doc). Checking each segment independently keeps the intent
/// (something in the path signals a test file) correct on every host OS.
pub(crate) fn looks_like_test_path(relative_path: &str) -> bool {
    relative_path.split('\\').any(looks_like_test_segment)
}

fn looks_like_test_segment(segment: &str) -> bool {
    let lower = segment.to_lowercase();
    let is_boundary = |byte: u8| matches!(byte, b'.' | b'_' | b'-');
    for candidate in ["test", "spec"] {
        for (index, _) in lower.match_indices(candidate) {
            let before_ok = index == 0 || is_boundary(lower.as_bytes()[index - 1]);
            let after_index = index + candidate.len();
            let after_ok = after_index == lower.len() || is_boundary(lower.as_bytes()[after_index]);
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_of_returns_last_segment() {
        assert_eq!(file_name_of("src\\utils\\main.py"), "main.py");
        assert_eq!(file_name_of("main.py"), "main.py");
    }

    #[test]
    fn path_depth_counts_segments() {
        assert_eq!(path_depth("main.py"), 1);
        assert_eq!(path_depth("src\\utils\\main.py"), 3);
    }

    #[test]
    fn extension_key_collapses_dockerfile_variants() {
        assert_eq!(extension_key("Dockerfile"), "dockerfile");
        assert_eq!(extension_key("docker\\Dockerfile.prod"), "dockerfile");
    }

    #[test]
    fn extension_key_handles_dotfiles_and_missing_extensions() {
        assert_eq!(extension_key(".env"), "[no extension]");
        assert_eq!(extension_key("README"), "[no extension]");
        assert_eq!(extension_key("src\\App.TSX"), "tsx");
    }

    #[test]
    fn to_native_path_rebuilds_components_regardless_of_host_os() {
        let native = to_native_path("src\\utils\\main.py");
        assert_eq!(native, Path::new("src").join("utils").join("main.py"));
    }

    #[test]
    fn resolve_joins_onto_root() {
        let root = Path::new("C:\\staging");
        let resolved = resolve(root, "src\\main.py");
        assert_eq!(resolved, root.join("src").join("main.py"));
    }

    #[test]
    fn looks_like_test_path_matches_test_segment() {
        assert!(looks_like_test_path("tests\\test_util.py"));
        assert!(looks_like_test_path("src\\App.test.tsx"));
        assert!(looks_like_test_path("spec\\login.spec.ts"));
    }

    #[test]
    fn looks_like_test_path_rejects_substring_without_boundary() {
        assert!(!looks_like_test_path("src\\protest.py"));
        assert!(!looks_like_test_path("src\\respected.py"));
    }
}
