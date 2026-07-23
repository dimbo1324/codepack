//! `uncommitted` mode: working-tree status (tracked changes + untracked new files),
//! ported from legacy `diff_service.py::_git_selection`'s `uncommitted` branch (`git
//! status --porcelain=v1 -z`) and its `_parse_porcelain_records` classification:
//! rename/copy status wins first, then deletion, then a bare `??` (genuinely
//! untracked) is "added", and everything else — including a staged-but-uncommitted
//! new file, which legacy's own parser does **not** special-case — falls through to
//! "modified". `git2`'s `Status` flags are mapped to preserve that exact priority.

use std::collections::HashSet;
use std::path::Path;

use git2::{Status, StatusOptions};

use super::git_common::{disabled_by_git_error, discover_repository, git_error_message};
use super::{DiffFile, DiffSelection, FileStatus};
use crate::normalize::to_backslash_path;

pub fn uncommitted_selection(source_root: &Path) -> DiffSelection {
    let repo = match discover_repository(source_root) {
        Ok(repo) => repo,
        Err(selection) => return *selection,
    };

    let mut status_opts = StatusOptions::new();
    status_opts
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = match repo.statuses(Some(&mut status_opts)) {
        Ok(statuses) => statuses,
        Err(err) => return disabled_by_git_error(&git_error_message(&err)),
    };

    let mut files: Vec<DiffFile> = Vec::new();
    for entry in statuses.iter() {
        let status = entry.status();

        if status.contains(Status::WT_RENAMED) || status.contains(Status::INDEX_RENAMED) {
            // `StatusEntry::path()` reports the delta's *old* path for a rename (a
            // documented git2 quirk — it reads `head_to_index`/`index_to_workdir`'s
            // `old_file.path`), so the new/current path must come from the delta
            // itself rather than the generic `entry.path()` used below.
            let Some(delta) = entry.head_to_index().or_else(|| entry.index_to_workdir()) else {
                continue;
            };
            let Some(new_path) = delta.new_file().path().map(to_backslash_path) else {
                continue;
            };
            let old_path = delta.old_file().path().map(to_backslash_path);
            files.push(DiffFile {
                relative_path: new_path,
                status: FileStatus::Renamed,
                old_path,
            });
            continue;
        }

        let Ok(path) = entry.path() else { continue };
        let relative_path = to_backslash_path(Path::new(path));

        if status.contains(Status::WT_DELETED) || status.contains(Status::INDEX_DELETED) {
            files.push(DiffFile {
                relative_path,
                status: FileStatus::Deleted,
                old_path: None,
            });
            continue;
        }
        if status.contains(Status::WT_NEW) && !status.contains(Status::INDEX_NEW) {
            files.push(DiffFile {
                relative_path,
                status: FileStatus::Added,
                old_path: None,
            });
            continue;
        }
        files.push(DiffFile {
            relative_path,
            status: FileStatus::Modified,
            old_path: None,
        });
    }
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let paths: HashSet<String> = files
        .iter()
        .filter(|item| item.status != FileStatus::Deleted)
        .map(|item| item.relative_path.clone())
        .collect();

    DiffSelection {
        mode: "uncommitted".to_string(),
        base: "незакоммиченные изменения".to_string(),
        paths: Some(paths),
        files,
        warning: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::fs;

    fn init_repo() -> (tempfile::TempDir, Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        (dir, repo)
    }

    fn commit_all(repo: &Repository, message: &str) {
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = Signature::now("Test", "test@example.local").unwrap();
        let parents: Vec<git2::Commit<'_>> =
            match repo.head().ok().and_then(|h| h.peel_to_commit().ok()) {
                Some(parent) => vec![parent],
                None => vec![],
            };
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent_refs,
        )
        .unwrap();
    }

    #[test]
    fn includes_untracked_and_modified_tracked_files() {
        let (dir, repo) = init_repo();
        fs::write(dir.path().join("tracked.py"), "print(1)").unwrap();
        commit_all(&repo, "init");

        fs::write(dir.path().join("tracked.py"), "print(2)").unwrap();
        fs::write(dir.path().join("new.py"), "print(3)").unwrap();

        let selection = uncommitted_selection(dir.path());

        assert_eq!(
            selection.paths,
            Some(
                ["tracked.py".to_string(), "new.py".to_string()]
                    .into_iter()
                    .collect()
            )
        );
    }

    #[test]
    fn handles_paths_with_spaces_and_unicode() {
        let (dir, _repo) = init_repo();
        fs::write(dir.path().join("file with space имя.py"), "print(1)").unwrap();

        let selection = uncommitted_selection(dir.path());

        assert_eq!(
            selection.paths,
            Some(["file with space имя.py".to_string()].into_iter().collect())
        );
    }

    #[test]
    fn detects_rename_preserving_old_path() {
        let (dir, repo) = init_repo();
        fs::write(
            dir.path().join("old name.py"),
            "print(1)\nprint(2)\nprint(3)\n",
        )
        .unwrap();
        commit_all(&repo, "init");

        fs::rename(
            dir.path().join("old name.py"),
            dir.path().join("new name.py"),
        )
        .unwrap();
        // Stage the rename so git2's status machinery can classify it as Renamed
        // rather than a delete+add pair (mirrors legacy's `git mv` in its own test).
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("old name.py")).unwrap();
        index.add_path(Path::new("new name.py")).unwrap();
        index.write().unwrap();

        let selection = uncommitted_selection(dir.path());

        assert_eq!(
            selection.paths,
            Some(["new name.py".to_string()].into_iter().collect())
        );
        let renamed = selection
            .files
            .iter()
            .find(|item| item.status == FileStatus::Renamed)
            .unwrap();
        assert_eq!(renamed.old_path.as_deref(), Some("old name.py"));
    }

    #[test]
    fn detects_deleted_tracked_file() {
        let (dir, repo) = init_repo();
        fs::write(dir.path().join("gone.py"), "print(1)").unwrap();
        commit_all(&repo, "init");

        fs::remove_file(dir.path().join("gone.py")).unwrap();

        let selection = uncommitted_selection(dir.path());

        let deleted: Vec<&DiffFile> = selection.deleted().collect();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].relative_path, "gone.py");
        assert!(!selection.paths.as_ref().unwrap().contains("gone.py"));
    }

    #[test]
    fn falls_back_to_all_with_warning_outside_a_repository() {
        let dir = tempfile::tempdir().unwrap();

        let selection = uncommitted_selection(dir.path());

        assert_eq!(selection.mode, "all");
        assert!(!selection.is_limited());
        assert!(selection.warning.is_some());
    }
}
