//! [`DiffSelection`]/[`DiffFile`]/[`FileStatus`] and [`resolve_diff_selection`], the
//! mode dispatcher ported from legacy `diff_service.py::resolve_diff_selection`.

mod all;
mod git_common;
mod git_ref;
mod last_export;
mod uncommitted;

use std::collections::HashSet;
use std::path::Path;

use codepack_core::CancellationToken;

pub use all::all_selection;
pub use git_ref::git_ref_selection;
pub use last_export::diff_against_snapshot;
pub use uncommitted::uncommitted_selection;

use crate::error::Result;
use crate::options::DiffOptions;
use crate::snapshot::{Snapshot, snapshot_project};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl FileStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            FileStatus::Added => "added",
            FileStatus::Modified => "modified",
            FileStatus::Deleted => "deleted",
            FileStatus::Renamed => "renamed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffFile {
    pub relative_path: String,
    pub status: FileStatus,
    pub old_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSelection {
    pub mode: String,
    pub base: String,
    pub paths: Option<HashSet<String>>,
    pub files: Vec<DiffFile>,
    pub warning: Option<String>,
}

impl DiffSelection {
    pub fn is_limited(&self) -> bool {
        self.paths.is_some()
    }

    pub fn deleted(&self) -> impl Iterator<Item = &DiffFile> {
        self.files
            .iter()
            .filter(|item| item.status == FileStatus::Deleted)
    }
}

/// Ported from legacy `resolve_diff_selection`. `previous_snapshot` is the caller's
/// baseline for `last_export` mode — this crate never looks one up itself (S5's job).
/// `ignored_dir_names`/`is_countable_text` flow straight through to
/// [`snapshot_project`] for the same reason `snapshot_project` itself takes them: no
/// hardcoded second copy of those constant sets in this crate.
pub fn resolve_diff_selection<F>(
    source_root: &Path,
    options: &DiffOptions,
    previous_snapshot: Option<&Snapshot>,
    ignored_dir_names: &HashSet<String>,
    is_countable_text: &F,
    cancel: &CancellationToken,
) -> Result<DiffSelection>
where
    F: Fn(&Path) -> bool + Sync,
{
    match options.mode.as_str() {
        "all" => Ok(all_selection()),
        "last_export" => {
            let current =
                snapshot_project(source_root, ignored_dir_names, is_countable_text, cancel)?;
            Ok(diff_against_snapshot(&current, previous_snapshot))
        }
        "git_ref" => Ok(git_ref_selection(source_root, &options.base_ref)),
        "uncommitted" => Ok(uncommitted_selection(source_root)),
        other => Ok(DiffSelection {
            mode: "all".to_string(),
            base: "полный экспорт".to_string(),
            paths: None,
            files: Vec::new(),
            warning: Some(format!(
                "Неизвестный режим дифференциального экспорта: {other}"
            )),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::SnapshotFile;

    fn options(mode: &str) -> DiffOptions {
        DiffOptions {
            mode: mode.to_string(),
            base_ref: "HEAD".to_string(),
        }
    }

    fn always_text(_: &Path) -> bool {
        true
    }

    #[test]
    fn unknown_mode_falls_back_to_all_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        let selection = resolve_diff_selection(
            dir.path(),
            &options("not_a_real_mode"),
            None,
            &HashSet::new(),
            &always_text,
            &CancellationToken::new(),
        )
        .unwrap();

        assert_eq!(selection.mode, "all");
        assert!(!selection.is_limited());
        assert!(selection.warning.is_some());
    }

    #[test]
    fn all_mode_has_no_path_filter() {
        let dir = tempfile::tempdir().unwrap();
        let selection = resolve_diff_selection(
            dir.path(),
            &options("all"),
            None,
            &HashSet::new(),
            &always_text,
            &CancellationToken::new(),
        )
        .unwrap();

        assert!(!selection.is_limited());
        assert!(selection.files.is_empty());
    }

    #[test]
    fn last_export_mode_snapshots_and_diffs_against_the_caller_supplied_baseline() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "one").unwrap();

        let previous = Snapshot {
            files: [(
                "a.txt".to_string(),
                SnapshotFile {
                    rel_path: "a.txt".to_string(),
                    sha256: "different".to_string(),
                    size: 3,
                    loc: 1,
                    mtime_ns: 0,
                },
            )]
            .into_iter()
            .collect(),
        };

        let selection = resolve_diff_selection(
            dir.path(),
            &options("last_export"),
            Some(&previous),
            &HashSet::new(),
            &always_text,
            &CancellationToken::new(),
        )
        .unwrap();

        assert!(selection.paths.as_ref().unwrap().contains("a.txt"));
        assert!(selection.warning.is_none());
    }
}
