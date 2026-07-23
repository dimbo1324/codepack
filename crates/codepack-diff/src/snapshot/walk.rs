//! Walks a project directory producing [`Snapshot`] entries, ported from legacy
//! `diff_service.py::snapshot_project`. Mirrors `codepack-scanner::walk.rs`'s
//! discipline: `walkdir` with `follow_links(false)`, an explicit `path_is_symlink()`
//! recheck (invariant I7 — never rely solely on the walker default), a sequential
//! collection pass that checks the [`CancellationToken`] on every iteration, and a
//! `rayon` parallel pass for the per-file work (here: hashing + LOC counting, both
//! more expensive than the scanner's plain `stat`).
//!
//! The ignored-directory-name set is entirely caller-supplied (lower-cased); this
//! module has no built-in `IGNORED_DIR_NAMES` of its own (see `lib.rs`'s scope
//! boundary — this is the deliberate resolution of the Q7-shaped constants-duplication
//! tension, `docs/decisions/open-questions.md` Q7).
//!
//! A file that cannot be `stat`-ed or hashed is silently skipped, matching legacy's
//! bare `except Exception: continue` inside the per-file loop. A `WalkDir` iteration
//! error (e.g. permission denied reading a subdirectory) is instead propagated as
//! `DiffError::Walk`, matching the precedent `codepack-scanner::walk.rs` already set
//! for this codebase (a documented, intentional deviation from Python's
//! `os.walk(onerror=None)` default of silently ignoring such errors).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use codepack_core::CancellationToken;
use rayon::prelude::*;
use walkdir::WalkDir;

use super::hash::hash_file;
use super::loc::count_loc_if_countable;
use super::{Snapshot, SnapshotFile};
use crate::error::{DiffError, Result};
use crate::normalize::to_backslash_path;

/// `ignored_dir_names` must already be lower-cased by the caller; matched
/// case-insensitively against each directory's basename, mirroring legacy
/// `should_ignore_dir`'s `name.casefold() in defaults` check.
pub fn snapshot_project<F>(
    root: &Path,
    ignored_dir_names: &HashSet<String>,
    is_countable_text: &F,
    cancel: &CancellationToken,
) -> Result<Snapshot>
where
    F: Fn(&Path) -> bool + Sync,
{
    let mut candidates: Vec<PathBuf> = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            if !entry.file_type().is_dir() {
                return !entry.path_is_symlink();
            }
            if entry.path_is_symlink() {
                return false;
            }
            let name_lower = entry.file_name().to_string_lossy().to_lowercase();
            !ignored_dir_names.contains(&name_lower)
        });

    for entry_result in walker {
        if cancel.is_cancelled() {
            return Err(DiffError::Cancelled);
        }
        let entry = entry_result.map_err(|source| DiffError::Walk {
            path: source
                .path()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root.to_path_buf()),
            source,
        })?;
        if entry.depth() == 0 || entry.file_type().is_dir() {
            continue;
        }
        if entry.path_is_symlink() {
            continue;
        }
        candidates.push(relative_path_of(entry.path(), root));
    }

    if cancel.is_cancelled() {
        return Err(DiffError::Cancelled);
    }

    let cancel_flag = cancel.clone();
    let entries: Vec<Option<(String, SnapshotFile)>> = candidates
        .par_iter()
        .map(|rel| {
            if cancel_flag.is_cancelled() {
                return None;
            }
            snapshot_one_file(root, rel, is_countable_text)
        })
        .collect();

    if cancel.is_cancelled() {
        return Err(DiffError::Cancelled);
    }

    let mut files: HashMap<String, SnapshotFile> = HashMap::new();
    for entry in entries.into_iter().flatten() {
        let (rel_path, snapshot_file) = entry;
        files.insert(rel_path, snapshot_file);
    }

    Ok(Snapshot { files })
}

fn snapshot_one_file<F>(
    root: &Path,
    rel: &Path,
    is_countable_text: &F,
) -> Option<(String, SnapshotFile)>
where
    F: Fn(&Path) -> bool,
{
    let full = root.join(rel);
    let metadata = fs::symlink_metadata(&full).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }
    let sha256 = hash_file(&full).ok()?;
    let loc = count_loc_if_countable(&full, is_countable_text);
    let rel_path = to_backslash_path(rel);
    let snapshot_file = SnapshotFile {
        rel_path: rel_path.clone(),
        sha256,
        size: metadata.len(),
        loc,
        mtime_ns: mtime_ns_of(&metadata),
    };
    Some((rel_path, snapshot_file))
}

fn mtime_ns_of(metadata: &fs::Metadata) -> i64 {
    // `modified()` is `Ok` on Windows/macOS/Linux (the three CI targets); a platform
    // lacking this timestamp falls back to the epoch rather than failing the whole
    // snapshot over a field that (unlike `sha256`) this crate's own diff logic never
    // compares — it exists only to match the `SNAPSHOT_FILE.mtime_ns` schema column
    // for a future consumer (BLUEPRINT §D.2).
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    match modified.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX),
        Err(err) => -i64::try_from(err.duration().as_nanos()).unwrap_or(i64::MAX),
    }
}

fn relative_path_of(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self as std_fs, File};

    fn no_ignored() -> HashSet<String> {
        HashSet::new()
    }

    fn always_text(_: &Path) -> bool {
        true
    }

    fn cancel_never() -> CancellationToken {
        CancellationToken::new()
    }

    #[test]
    fn snapshot_project_hashes_and_sizes_files() {
        let dir = tempfile::tempdir().unwrap();
        std_fs::write(dir.path().join("a.txt"), "hello").unwrap();

        let snapshot =
            snapshot_project(dir.path(), &no_ignored(), &always_text, &cancel_never()).unwrap();

        assert_eq!(snapshot.len(), 1);
        let file = snapshot.files.get("a.txt").unwrap();
        assert_eq!(file.size, 5);
        assert_eq!(file.loc, 1);
        assert!(!file.sha256.is_empty());
    }

    #[test]
    fn snapshot_project_prunes_ignored_directories_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        std_fs::create_dir_all(dir.path().join("Node_Modules")).unwrap();
        File::create(dir.path().join("Node_Modules/pkg.js")).unwrap();
        File::create(dir.path().join("main.py")).unwrap();

        let mut ignored = HashSet::new();
        ignored.insert("node_modules".to_string());

        let snapshot =
            snapshot_project(dir.path(), &ignored, &always_text, &cancel_never()).unwrap();

        assert_eq!(snapshot.len(), 1);
        assert!(snapshot.files.contains_key("main.py"));
    }

    #[test]
    fn snapshot_project_backslash_normalizes_nested_paths() {
        let dir = tempfile::tempdir().unwrap();
        std_fs::create_dir_all(dir.path().join("src")).unwrap();
        std_fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

        let snapshot =
            snapshot_project(dir.path(), &no_ignored(), &always_text, &cancel_never()).unwrap();

        assert!(snapshot.files.contains_key("src\\main.rs"));
        assert!(!snapshot.files.contains_key("src/main.rs"));
    }

    #[test]
    fn snapshot_project_respects_cancellation() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            std_fs::write(dir.path().join(format!("file{i}.txt")), "x").unwrap();
        }
        let cancel = CancellationToken::new();
        cancel.cancel();

        let result = snapshot_project(dir.path(), &no_ignored(), &always_text, &cancel);

        assert!(matches!(result, Err(DiffError::Cancelled)));
    }

    #[test]
    fn snapshot_project_never_descends_into_a_directory_symlink() {
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

        let snapshot =
            snapshot_project(dir.path(), &no_ignored(), &always_text, &cancel_never()).unwrap();

        assert_eq!(snapshot.len(), 1);
        assert!(snapshot.files.contains_key("inside.txt"));
        assert!(!snapshot.files.keys().any(|k| k.contains("outside_secret")));
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
