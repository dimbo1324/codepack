//! Shared "not a repository / Git error" fallback, ported from legacy
//! `diff_service.py::_disabled_by_git_error`. `git_ref` and `uncommitted` never return
//! a hard error to the caller — any Git failure degrades to the `all` selection with a
//! warning, matching legacy's behavior exactly (a project the user chose to export
//! must still export even when its Git state cannot be read).

use git2::Repository;
use std::path::Path;

use super::DiffSelection;

pub(super) fn disabled_by_git_error(message: &str) -> DiffSelection {
    DiffSelection {
        mode: "all".to_string(),
        base: "полный экспорт".to_string(),
        paths: None,
        files: Vec::new(),
        warning: Some(format!("Дифференциальный Git-режим отключён: {message}")),
    }
}

/// `Repository::discover` walks upward from `source_root` looking for a `.git`
/// directory, mirroring legacy's `git rev-parse --is-inside-work-tree` run with
/// `cwd=source_root` (which also searches upward from the given directory).
///
/// The `Err` is boxed (`clippy::result_large_err`): `DiffSelection` carries a `Vec`/
/// `HashSet`/`String` fields, too large to return by value on the error path.
pub(super) fn discover_repository(source_root: &Path) -> Result<Repository, Box<DiffSelection>> {
    Repository::discover(source_root)
        .map_err(|err| Box::new(disabled_by_git_error(&git_error_message(&err))))
}

pub(super) fn git_error_message(err: &git2::Error) -> String {
    err.message().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_repository_fails_outside_any_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        let result = discover_repository(dir.path());
        assert!(result.is_err());
        if let Err(selection) = result {
            assert_eq!(selection.mode, "all");
            assert!(selection.warning.as_ref().unwrap().contains("Git"));
        }
    }
}
