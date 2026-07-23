//! Directory traversal: base-ignore and stack-ignore pruning, never following
//! symlinks (invariant I7). Ported from the top-down pruning half of legacy
//! `services/export_plan.py::build_export_plan`'s `os.walk` loop.
//!
//! The sequential directory walk collects file candidates (checking the
//! [`CancellationToken`] on every iteration); a `rayon` pass afterward stats each
//! candidate in parallel. `.exportignore`/custom-rule pruning is layered on top by the
//! caller via `extra_dir_filter` — this module only knows about
//! [`constants::IGNORED_DIR_NAMES`], stack-detected directories, and `Config`'s
//! `extra_ignored_dirs`, matching the S2 scope boundary (`lib.rs`).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::constants;
use crate::error::{Result, ScannerError};
use crate::ignore::pattern::{GlobPattern, has_glob_metachars};

/// Base `IGNORED_DIR_NAMES` plus caller-supplied extra directory names (config +
/// stack-detected), matched case-insensitively against a directory's basename. Most
/// entries are exact literal names; an entry containing a glob metacharacter (the one
/// documented case being Python's stack rule `*.egg-info`) is basename-glob-matched
/// instead — legacy's own runtime `should_ignore_dir` never actually glob-matches that
/// entry (it is a plain `in a_set` membership test), which makes it dead in practice;
/// this scanner intentionally fixes that so the directory is really pruned, an
/// explicit, documented improvement over the literal legacy behavior.
#[derive(Debug, Clone, Default)]
pub struct IgnoredDirMatcher {
    literal: HashSet<String>,
    globs: Vec<GlobPattern>,
}

impl IgnoredDirMatcher {
    pub fn new(extra_names: impl IntoIterator<Item = String>) -> Self {
        let mut literal = HashSet::new();
        let mut globs = Vec::new();
        for raw in extra_names {
            let lower = raw.trim().to_lowercase();
            if lower.is_empty() {
                continue;
            }
            if has_glob_metachars(&lower) {
                globs.push(GlobPattern::new(&lower));
            } else {
                literal.insert(lower);
            }
        }
        Self { literal, globs }
    }

    pub fn is_ignored(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        constants::is_base_ignored_dir_name(&lower)
            || self.literal.contains(&lower)
            || self.globs.iter().any(|pattern| pattern.matches(&lower))
    }
}

#[derive(Debug, Clone)]
pub struct WalkedFile {
    pub relative_path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct SkippedDir {
    pub relative_path: PathBuf,
    pub reason: Option<String>,
}

#[derive(Debug, Default)]
pub struct WalkOutcome {
    pub files: Vec<WalkedFile>,
    pub skipped_dirs: Vec<SkippedDir>,
}

/// Walks `root`, pruning symlinked directories and anything `ignored_dirs` matches
/// before recursing into them. `extra_dir_filter` is consulted only for directories
/// that already survived that base check, letting the caller layer
/// `.exportignore`/custom-rule pruning (or, in a future stage, safety-mode pruning) on
/// top without this module knowing about those rule types.
pub fn walk_project(
    root: &Path,
    ignored_dirs: &IgnoredDirMatcher,
    cancel: &CancellationToken,
    mut extra_dir_filter: impl FnMut(&Path) -> Option<String>,
) -> Result<WalkOutcome> {
    let mut skipped_dirs: Vec<SkippedDir> = Vec::new();
    let mut candidate_files: Vec<PathBuf> = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            if !entry.file_type().is_dir() {
                // Symlinked files are silently dropped, matching legacy's bare
                // `if path.is_symlink(): continue` (no skipped_dirs/excluded_files
                // entry is ever recorded for them).
                return !entry.path_is_symlink();
            }
            let rel = relative_path_of(entry.path(), root);
            // `follow_links(false)` already keeps walkdir from descending into a
            // symlinked directory, but `path_is_symlink()` is checked independently
            // here too (invariant I7: never rely solely on the walker default).
            if entry.path_is_symlink()
                || ignored_dirs.is_ignored(&entry.file_name().to_string_lossy())
            {
                skipped_dirs.push(SkippedDir {
                    relative_path: rel,
                    reason: None,
                });
                return false;
            }
            if let Some(reason) = extra_dir_filter(&rel) {
                skipped_dirs.push(SkippedDir {
                    relative_path: rel,
                    reason: Some(reason),
                });
                return false;
            }
            true
        });

    for entry_result in walker {
        if cancel.is_cancelled() {
            return Err(ScannerError::Cancelled);
        }
        let entry = entry_result.map_err(|source| ScannerError::Walk {
            path: source
                .path()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root.to_path_buf()),
            source,
        })?;
        if entry.depth() == 0 || entry.file_type().is_dir() {
            continue;
        }
        candidate_files.push(relative_path_of(entry.path(), root));
    }

    let cancel_flag = cancel.clone();
    let files: Vec<WalkedFile> = candidate_files
        .par_iter()
        .map(|rel| {
            if cancel_flag.is_cancelled() {
                return WalkedFile {
                    relative_path: rel.clone(),
                    size: 0,
                };
            }
            let full = root.join(rel);
            let size = fs::metadata(&full).map(|meta| meta.len()).unwrap_or(0);
            WalkedFile {
                relative_path: rel.clone(),
                size,
            }
        })
        .collect();
    if cancel.is_cancelled() {
        return Err(ScannerError::Cancelled);
    }

    Ok(WalkOutcome {
        files,
        skipped_dirs,
    })
}

fn relative_path_of(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self as std_fs, File};

    fn cancel_never() -> CancellationToken {
        CancellationToken::new()
    }

    #[test]
    fn ignored_dir_matcher_matches_base_names_case_insensitively() {
        let matcher = IgnoredDirMatcher::new(std::iter::empty());
        assert!(matcher.is_ignored("node_modules"));
        assert!(matcher.is_ignored("NODE_MODULES"));
        assert!(!matcher.is_ignored("src"));
    }

    #[test]
    fn ignored_dir_matcher_matches_extra_literal_names() {
        let matcher = IgnoredDirMatcher::new(["vendor".to_string()]);
        assert!(matcher.is_ignored("vendor"));
        assert!(!matcher.is_ignored("target"));
    }

    #[test]
    fn ignored_dir_matcher_glob_matches_egg_info_style_entries() {
        let matcher = IgnoredDirMatcher::new(["*.egg-info".to_string()]);
        assert!(matcher.is_ignored("my_package.egg-info"));
        assert!(!matcher.is_ignored("my_package"));
    }

    #[test]
    fn walk_project_prunes_base_ignored_directories() {
        let dir = tempfile::tempdir().unwrap();
        std_fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
        File::create(dir.path().join("node_modules/pkg/index.js")).unwrap();
        File::create(dir.path().join("main.py")).unwrap();

        let matcher = IgnoredDirMatcher::new(std::iter::empty());
        let outcome = walk_project(dir.path(), &matcher, &cancel_never(), |_| None).unwrap();

        assert_eq!(outcome.files.len(), 1);
        assert_eq!(outcome.files[0].relative_path, Path::new("main.py"));
        assert_eq!(outcome.skipped_dirs.len(), 1);
        assert_eq!(
            outcome.skipped_dirs[0].relative_path,
            Path::new("node_modules")
        );
        assert!(outcome.skipped_dirs[0].reason.is_none());
    }

    #[test]
    fn walk_project_reports_file_sizes() {
        let dir = tempfile::tempdir().unwrap();
        std_fs::write(dir.path().join("data.txt"), "hello world").unwrap();

        let matcher = IgnoredDirMatcher::new(std::iter::empty());
        let outcome = walk_project(dir.path(), &matcher, &cancel_never(), |_| None).unwrap();

        assert_eq!(outcome.files.len(), 1);
        assert_eq!(outcome.files[0].size, 11);
    }

    #[test]
    fn walk_project_applies_extra_dir_filter_and_records_its_reason() {
        let dir = tempfile::tempdir().unwrap();
        std_fs::create_dir_all(dir.path().join("private")).unwrap();
        File::create(dir.path().join("private/secret.txt")).unwrap();

        let matcher = IgnoredDirMatcher::new(std::iter::empty());
        let outcome = walk_project(dir.path(), &matcher, &cancel_never(), |rel| {
            if rel == Path::new("private") {
                Some("custom rule".to_string())
            } else {
                None
            }
        })
        .unwrap();

        assert!(outcome.files.is_empty());
        assert_eq!(outcome.skipped_dirs.len(), 1);
        assert_eq!(
            outcome.skipped_dirs[0].reason.as_deref(),
            Some("custom rule")
        );
    }

    #[test]
    fn walk_project_respects_cancellation() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            File::create(dir.path().join(format!("file{i}.txt"))).unwrap();
        }
        let cancel = CancellationToken::new();
        cancel.cancel();
        let matcher = IgnoredDirMatcher::new(std::iter::empty());
        let result = walk_project(dir.path(), &matcher, &cancel, |_| None);
        assert!(matches!(result, Err(ScannerError::Cancelled)));
    }

    #[test]
    fn walk_project_never_descends_into_a_directory_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        File::create(outside.path().join("outside_secret.txt")).unwrap();
        File::create(dir.path().join("inside.txt")).unwrap();

        let link_path = dir.path().join("escape");
        let created = create_dir_symlink(outside.path(), &link_path);
        if !created {
            eprintln!(
                "skipping symlink-escape assertion: this environment cannot create directory \
                 symlinks (no Developer Mode / admin privilege on Windows)"
            );
            return;
        }

        let matcher = IgnoredDirMatcher::new(std::iter::empty());
        let outcome = walk_project(dir.path(), &matcher, &cancel_never(), |_| None).unwrap();

        assert!(
            outcome
                .files
                .iter()
                .all(|f| f.relative_path != Path::new("escape/outside_secret.txt"))
        );
        assert_eq!(outcome.files.len(), 1);
        assert_eq!(outcome.files[0].relative_path, Path::new("inside.txt"));
    }

    #[cfg(unix)]
    fn create_dir_symlink(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    #[cfg(windows)]
    fn create_dir_symlink(target: &Path, link: &Path) -> bool {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    }

    #[cfg(not(any(unix, windows)))]
    fn create_dir_symlink(_target: &Path, _link: &Path) -> bool {
        false
    }
}
