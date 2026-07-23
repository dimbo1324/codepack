//! `last_export` mode: compares a current [`Snapshot`] against the previous
//! successful export's snapshot, ported from legacy
//! `diff_service.py::_selection_from_snapshots`. The previous snapshot is a plain
//! function argument (`Option<&Snapshot>`) — this crate never looks one up from a
//! history store itself (that lookup is `codepack-storage`'s job, stage S5).

use std::collections::HashSet;

use super::{DiffFile, DiffSelection, FileStatus};
use crate::snapshot::Snapshot;

/// Verbatim legacy warning text (`diff_service.py`'s Russian message) — this crate's
/// artifacts are user-facing, not agent-facing infrastructure, so the language policy
/// that mandates English does not apply here (see `report.rs`'s doc comment).
const NO_PREVIOUS_SNAPSHOT_WARNING: &str =
    "Предыдущий экспорт для проекта не найден, поэтому выбран весь текущий проект.";

pub fn diff_against_snapshot(current: &Snapshot, previous: Option<&Snapshot>) -> DiffSelection {
    let Some(previous) = previous else {
        let mut relative_paths: Vec<&String> = current.files.keys().collect();
        relative_paths.sort();
        let files: Vec<DiffFile> = relative_paths
            .into_iter()
            .map(|rel| DiffFile {
                relative_path: rel.clone(),
                status: FileStatus::Added,
                old_path: None,
            })
            .collect();
        let paths: HashSet<String> = current.files.keys().cloned().collect();
        return DiffSelection {
            mode: "last_export".to_string(),
            base: "последний экспорт".to_string(),
            paths: Some(paths),
            files,
            warning: Some(NO_PREVIOUS_SNAPSHOT_WARNING.to_string()),
        };
    };

    let mut changed: Vec<DiffFile> = Vec::new();
    for (rel, meta) in &current.files {
        match previous.files.get(rel) {
            None => changed.push(DiffFile {
                relative_path: rel.clone(),
                status: FileStatus::Added,
                old_path: None,
            }),
            Some(old) if old.sha256 != meta.sha256 => changed.push(DiffFile {
                relative_path: rel.clone(),
                status: FileStatus::Modified,
                old_path: None,
            }),
            _ => {}
        }
    }
    for rel in previous.files.keys() {
        if !current.files.contains_key(rel) {
            changed.push(DiffFile {
                relative_path: rel.clone(),
                status: FileStatus::Deleted,
                old_path: None,
            });
        }
    }
    changed.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let paths: HashSet<String> = changed
        .iter()
        .filter(|item| item.status != FileStatus::Deleted)
        .map(|item| item.relative_path.clone())
        .collect();

    DiffSelection {
        mode: "last_export".to_string(),
        base: "последний экспорт".to_string(),
        paths: Some(paths),
        files: changed,
        warning: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::SnapshotFile;

    fn file(rel: &str, sha: &str) -> SnapshotFile {
        SnapshotFile {
            rel_path: rel.to_string(),
            sha256: sha.to_string(),
            size: 1,
            loc: 1,
            mtime_ns: 0,
        }
    }

    fn snapshot(entries: &[(&str, &str)]) -> Snapshot {
        Snapshot {
            files: entries
                .iter()
                .map(|(rel, sha)| (rel.to_string(), file(rel, sha)))
                .collect(),
        }
    }

    #[test]
    fn no_previous_snapshot_selects_everything_as_added_with_a_warning() {
        let current = snapshot(&[("a.py", "sha-a"), ("b.py", "sha-b")]);

        let selection = diff_against_snapshot(&current, None);

        assert_eq!(
            selection.paths,
            Some(
                ["a.py".to_string(), "b.py".to_string()]
                    .into_iter()
                    .collect()
            )
        );
        assert!(
            selection
                .files
                .iter()
                .all(|item| item.status == FileStatus::Added)
        );
        assert!(selection.warning.is_some());
    }

    #[test]
    fn detects_added_modified_and_deleted() {
        let previous = snapshot(&[("a.py", "old-a"), ("gone.py", "old-gone")]);
        let current = snapshot(&[("a.py", "new-a"), ("b.py", "new-b")]);

        let selection = diff_against_snapshot(&current, Some(&previous));

        assert_eq!(
            selection.paths,
            Some(
                ["a.py".to_string(), "b.py".to_string()]
                    .into_iter()
                    .collect()
            )
        );
        let statuses: HashSet<(String, FileStatus)> = selection
            .files
            .iter()
            .map(|item| (item.relative_path.clone(), item.status))
            .collect();
        assert!(statuses.contains(&("a.py".to_string(), FileStatus::Modified)));
        assert!(statuses.contains(&("b.py".to_string(), FileStatus::Added)));
        assert!(statuses.contains(&("gone.py".to_string(), FileStatus::Deleted)));
        assert!(selection.warning.is_none());
    }

    #[test]
    fn unchanged_files_produce_no_diff_entries() {
        let previous = snapshot(&[("stable.py", "same")]);
        let current = snapshot(&[("stable.py", "same")]);

        let selection = diff_against_snapshot(&current, Some(&previous));

        assert_eq!(selection.paths, Some(HashSet::new()));
        assert!(selection.files.is_empty());
    }
}
