//! `build_export_paths()`: derives every path the export pipeline writes to, ported
//! from legacy `utils/path_utils.py::build_export_paths`/`sanitize_name`.
//!
//! Legacy hardcodes the output root to `Path.home() / "Desktop"` (falling back to the
//! home directory itself when Desktop does not exist). This crate takes that root as an
//! explicit `output_root` parameter instead — default-root resolution (Desktop-or-
//! fallback) is a CLI/UI concern deferred to stage S10/S11 (`task-checklist.md`'s S9
//! design decision), not this crate's job. The `ExportPaths.desktop` field name is a
//! fixed contract from `codepack-core` and is populated with whatever `output_root` the
//! caller supplied, Desktop or otherwise.

use std::path::{Path, PathBuf};

use codepack_core::ExportPaths;

use crate::timestamp::compact_utc_stamp;

/// Ported from legacy `sanitize_name`: strips characters that are invalid in a
/// filename on at least one supported platform, trims whitespace, and falls back to
/// `"project"` when nothing usable remains.
pub fn sanitize_name(name: &str) -> String {
    const FORBIDDEN: &str = "<>:\"/\\|?*";
    let cleaned: String = name.chars().filter(|ch| !FORBIDDEN.contains(*ch)).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Builds every path the export pipeline needs, ported from legacy
/// `build_export_paths`. Collision handling (append `_1`, `_2`, ... to the bundle name
/// until none of `staging`/`final_zip`/`archive_set_dir` already exist) is preserved
/// verbatim.
pub fn build_export_paths(source_root: &Path, output_root: &Path) -> ExportPaths {
    build_export_paths_with_stamp(source_root, output_root, &compact_utc_stamp())
}

/// Test seam for [`build_export_paths`]: takes the bundle-name timestamp as a
/// parameter so collision handling can be exercised deterministically instead of
/// depending on two calls landing in the same wall-clock second.
fn build_export_paths_with_stamp(
    source_root: &Path,
    output_root: &Path,
    stamp: &str,
) -> ExportPaths {
    let raw_name = source_root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let project_name = sanitize_name(&raw_name);

    let base = format!("{project_name}_export_{stamp}");
    let mut bundle_name = base.clone();
    let candidate_paths = |name: &str| -> (PathBuf, PathBuf, PathBuf) {
        (
            output_root.join(name),
            output_root.join(format!("{name}.zip")),
            output_root.join(format!("{name}_archives")),
        )
    };

    let (mut staging, mut final_zip, mut archive_set_dir) = candidate_paths(&bundle_name);
    let mut counter: u32 = 1;
    while staging.exists() || final_zip.exists() || archive_set_dir.exists() {
        bundle_name = format!("{base}_{counter}");
        (staging, final_zip, archive_set_dir) = candidate_paths(&bundle_name);
        counter += 1;
    }

    let project_dir = staging.join(&project_name);
    let reports_dir = staging.join("reports");
    let insights_dir = reports_dir.join("insights");

    ExportPaths {
        desktop: output_root.to_path_buf(),
        source_root: source_root.to_path_buf(),
        project_name,
        bundle_name,
        staging_dir: staging.clone(),
        final_zip,
        archive_set_dir,
        project_dir,
        manifest_file: staging.join("manifest.json"),
        project_profile_file: staging.join("PROJECT_PROFILE.json"),
        index_file: staging.join("INDEX.md"),
        structure_report: reports_dir.join("01_structure.txt"),
        git_report: reports_dir.join("02_git.txt"),
        text_dump: reports_dir.join("03_text_dump.txt"),
        insights_dir,
        reports_dir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_name_strips_forbidden_characters() {
        assert_eq!(sanitize_name("my<project>:name"), "myprojectname");
    }

    #[test]
    fn sanitize_name_falls_back_to_project_when_empty() {
        assert_eq!(sanitize_name("???"), "project");
        assert_eq!(sanitize_name("   "), "project");
        assert_eq!(sanitize_name(""), "project");
    }

    #[test]
    fn sanitize_name_trims_surrounding_whitespace() {
        assert_eq!(sanitize_name("  my project  "), "my project");
    }

    #[test]
    fn build_export_paths_derives_the_expected_layout() {
        let output_root = tempfile::tempdir().unwrap();
        let source_root = Path::new("/some/where/my_project");

        let paths =
            build_export_paths_with_stamp(source_root, output_root.path(), "20240101_120000");

        assert_eq!(paths.project_name, "my_project");
        assert_eq!(paths.bundle_name, "my_project_export_20240101_120000");
        assert_eq!(paths.desktop, output_root.path());
        assert_eq!(paths.source_root, source_root);
        assert_eq!(
            paths.staging_dir,
            output_root.path().join("my_project_export_20240101_120000")
        );
        assert_eq!(
            paths.final_zip,
            output_root
                .path()
                .join("my_project_export_20240101_120000.zip")
        );
        assert_eq!(
            paths.archive_set_dir,
            output_root
                .path()
                .join("my_project_export_20240101_120000_archives")
        );
        assert_eq!(paths.project_dir, paths.staging_dir.join("my_project"));
        assert_eq!(paths.reports_dir, paths.staging_dir.join("reports"));
        assert_eq!(paths.insights_dir, paths.reports_dir.join("insights"));
        assert_eq!(paths.manifest_file, paths.staging_dir.join("manifest.json"));
        assert_eq!(
            paths.project_profile_file,
            paths.staging_dir.join("PROJECT_PROFILE.json")
        );
        assert_eq!(paths.index_file, paths.staging_dir.join("INDEX.md"));
        assert_eq!(
            paths.structure_report,
            paths.reports_dir.join("01_structure.txt")
        );
        assert_eq!(paths.git_report, paths.reports_dir.join("02_git.txt"));
        assert_eq!(paths.text_dump, paths.reports_dir.join("03_text_dump.txt"));
    }

    #[test]
    fn build_export_paths_falls_back_to_project_for_an_unsanitizable_name() {
        let output_root = tempfile::tempdir().unwrap();
        let source_root = Path::new("???");

        let paths =
            build_export_paths_with_stamp(source_root, output_root.path(), "20240101_120000");

        assert_eq!(paths.project_name, "project");
    }

    #[test]
    fn build_export_paths_appends_a_counter_on_collision() {
        let output_root = tempfile::tempdir().unwrap();
        let source_root = Path::new("/some/where/my_project");
        let stamp = "20240101_120000";

        let base_bundle = format!("my_project_export_{stamp}");
        std::fs::create_dir_all(output_root.path().join(&base_bundle)).unwrap();

        let paths = build_export_paths_with_stamp(source_root, output_root.path(), stamp);

        assert_eq!(paths.bundle_name, format!("{base_bundle}_1"));
    }

    #[test]
    fn build_export_paths_keeps_incrementing_past_multiple_collisions() {
        let output_root = tempfile::tempdir().unwrap();
        let source_root = Path::new("/some/where/my_project");
        let stamp = "20240101_120000";

        let base_bundle = format!("my_project_export_{stamp}");
        std::fs::create_dir_all(output_root.path().join(&base_bundle)).unwrap();
        std::fs::create_dir_all(output_root.path().join(format!("{base_bundle}_1"))).unwrap();
        // Collide on the final_zip file for attempt "_2" specifically.
        std::fs::write(
            output_root.path().join(format!("{base_bundle}_2.zip")),
            b"x",
        )
        .unwrap();

        let paths = build_export_paths_with_stamp(source_root, output_root.path(), stamp);

        assert_eq!(paths.bundle_name, format!("{base_bundle}_3"));
    }

    #[test]
    fn public_build_export_paths_uses_a_real_timestamp() {
        let output_root = tempfile::tempdir().unwrap();
        let source_root = Path::new("/some/where/my_project");

        let paths = build_export_paths(source_root, output_root.path());

        assert!(paths.bundle_name.starts_with("my_project_export_"));
    }
}
