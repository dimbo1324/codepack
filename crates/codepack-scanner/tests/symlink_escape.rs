//! Invariant I7: the walk never follows a symlink out of the source tree. A directory
//! symlink is not committed as a static fixture — symlinks are not reliably portable
//! through a `git` checkout across operating systems and `core.symlinks` settings — so
//! this creates one programmatically in a temporary directory, mirroring
//! `codepack-scanner`'s own `walk::tests::walk_project_never_descends_into_a_directory_symlink`
//! but exercised through the public `build_export_plan` API end to end.

use std::fs;
use std::path::Path;

use codepack_core::CancellationToken;
use codepack_scanner::{ExportIgnoreRules, ScanOptions, build_export_plan};

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

#[test]
fn build_export_plan_never_includes_files_reached_through_a_symlinked_directory() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("outside_secret.txt"), "leak me not").unwrap();
    fs::write(project.path().join("inside.py"), "print(1)").unwrap();

    let link_path = project.path().join("escape");
    if !create_dir_symlink(outside.path(), &link_path) {
        eprintln!(
            "skipping I7 symlink-escape assertion: this environment cannot create directory \
             symlinks (no Developer Mode / admin privilege on Windows)"
        );
        return;
    }

    let options = ScanOptions::default();
    let rules = ExportIgnoreRules::from_project_and_config(project.path(), &options);
    let plan =
        build_export_plan(project.path(), &options, &rules, &CancellationToken::new()).unwrap();

    let all_paths: Vec<&str> = plan
        .included_files
        .iter()
        .chain(plan.excluded_files.iter())
        .map(|f| f.relative_path.as_str())
        .collect();
    assert!(all_paths.iter().all(|p| !p.contains("outside_secret")));
    assert_eq!(plan.included_files.len(), 1);
    assert_eq!(plan.included_files[0].relative_path, "inside.py");
}
