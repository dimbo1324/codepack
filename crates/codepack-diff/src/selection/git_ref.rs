//! `git_ref` mode: `base..HEAD` name-status diff with rename detection, ported from
//! legacy `diff_service.py::_git_selection`'s non-`uncommitted` branch (`git diff
//! --name-status --find-renames {base}..HEAD --`).
//!
//! `base` always diffs against `HEAD` — never against a second, caller-chosen target
//! ref. This is legacy parity, not an oversight: `diff_target_ref`/`_target_ref` is a
//! confirmed-dead legacy field (see `options.rs`'s doc comment and
//! `task-checklist.md`'s S4 scope note).

use std::collections::HashSet;
use std::path::Path;

use git2::{Delta, DiffFindOptions, DiffOptions as Git2DiffOptions, Repository, Tree};

use super::git_common::{disabled_by_git_error, discover_repository, git_error_message};
use super::{DiffFile, DiffSelection, FileStatus};
use crate::normalize::to_backslash_path;

pub fn git_ref_selection(source_root: &Path, base_ref: &str) -> DiffSelection {
    let repo = match discover_repository(source_root) {
        Ok(repo) => repo,
        Err(selection) => {
            return *selection;
        }
    };

    let base = if base_ref.trim().is_empty() {
        "HEAD"
    } else {
        base_ref.trim()
    };

    let base_tree = match resolve_tree(&repo, base) {
        Ok(tree) => tree,
        Err(message) => {
            return disabled_by_git_error(&message);
        }
    };
    let head_tree = match resolve_tree(&repo, "HEAD") {
        Ok(tree) => tree,
        Err(message) => {
            return disabled_by_git_error(&message);
        }
    };

    let mut diff_opts = Git2DiffOptions::new();
    let mut diff =
        match repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut diff_opts)) {
            Ok(diff) => diff,
            Err(err) => {
                return disabled_by_git_error(&git_error_message(&err));
            }
        };

    let mut find_opts = DiffFindOptions::new();
    find_opts.renames(true);
    if let Err(err) = diff.find_similar(Some(&mut find_opts)) {
        return disabled_by_git_error(&git_error_message(&err));
    }

    let mut files: Vec<DiffFile> = Vec::new();
    for delta in diff.deltas() {
        let new_path = delta.new_file().path().map(to_backslash_path);
        let old_path = delta.old_file().path().map(to_backslash_path);
        match delta.status() {
            Delta::Deleted => {
                if let Some(path) = old_path {
                    files.push(DiffFile {
                        relative_path: path,
                        status: FileStatus::Deleted,
                        old_path: None,
                    });
                }
            }
            Delta::Renamed => {
                if let Some(path) = new_path {
                    files.push(DiffFile {
                        relative_path: path,
                        status: FileStatus::Renamed,
                        old_path,
                    });
                }
            }
            Delta::Added => {
                if let Some(path) = new_path {
                    files.push(DiffFile {
                        relative_path: path,
                        status: FileStatus::Added,
                        old_path: None,
                    });
                }
            }
            _ => {
                if let Some(path) = new_path.or(old_path) {
                    files.push(DiffFile {
                        relative_path: path,
                        status: FileStatus::Modified,
                        old_path: None,
                    });
                }
            }
        }
    }
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let paths: HashSet<String> = files
        .iter()
        .filter(|item| item.status != FileStatus::Deleted)
        .map(|item| item.relative_path.clone())
        .collect();

    DiffSelection {
        mode: "git_ref".to_string(),
        base: base.to_string(),
        paths: Some(paths),
        files,
        warning: None,
    }
}

fn resolve_tree<'repo>(repo: &'repo Repository, refname: &str) -> Result<Tree<'repo>, String> {
    let object = repo
        .revparse_single(refname)
        .map_err(|err| git_error_message(&err))?;
    let commit = object
        .peel_to_commit()
        .map_err(|err| git_error_message(&err))?;
    commit.tree().map_err(|err| git_error_message(&err))
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

    fn commit_all(repo: &Repository, message: &str) -> git2::Oid {
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
        .unwrap()
    }

    #[test]
    fn diffs_base_to_head_never_the_uncommitted_working_tree() {
        let (dir, repo) = init_repo();
        fs::write(dir.path().join("tracked.py"), "print(1)").unwrap();
        commit_all(&repo, "init");
        let base = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
            .to_string();

        fs::write(dir.path().join("committed.py"), "print(2)").unwrap();
        commit_all(&repo, "second");

        // Uncommitted working-tree edit after the second commit; git_ref must ignore it.
        fs::write(dir.path().join("tracked.py"), "print(3)").unwrap();

        let selection = git_ref_selection(dir.path(), &base);

        assert_eq!(selection.mode, "git_ref");
        assert_eq!(
            selection.paths,
            Some(["committed.py".to_string()].into_iter().collect())
        );
    }

    #[test]
    fn empty_base_ref_defaults_to_head() {
        let (dir, repo) = init_repo();
        fs::write(dir.path().join("a.py"), "print(1)").unwrap();
        commit_all(&repo, "init");

        let selection = git_ref_selection(dir.path(), "");

        assert_eq!(selection.base, "HEAD");
        assert_eq!(selection.paths, Some(HashSet::new()));
    }

    #[test]
    fn detects_renames_with_spaces_and_unicode() {
        let (dir, repo) = init_repo();
        fs::write(
            dir.path().join("old имя.py"),
            "print(1)\nprint(2)\nprint(3)\n",
        )
        .unwrap();
        commit_all(&repo, "init");
        let base = repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id()
            .to_string();

        fs::rename(dir.path().join("old имя.py"), dir.path().join("new имя.py")).unwrap();
        commit_all(&repo, "rename");

        let selection = git_ref_selection(dir.path(), &base);

        assert_eq!(
            selection.paths,
            Some(["new имя.py".to_string()].into_iter().collect())
        );
        let renamed = selection
            .files
            .iter()
            .find(|item| item.status == FileStatus::Renamed)
            .unwrap();
        assert_eq!(renamed.relative_path, "new имя.py");
        assert_eq!(renamed.old_path.as_deref(), Some("old имя.py"));
    }

    #[test]
    fn falls_back_to_all_with_warning_outside_a_repository() {
        let dir = tempfile::tempdir().unwrap();

        let selection = git_ref_selection(dir.path(), "HEAD");

        assert_eq!(selection.mode, "all");
        assert!(!selection.is_limited());
        assert!(selection.warning.is_some());
    }
}
