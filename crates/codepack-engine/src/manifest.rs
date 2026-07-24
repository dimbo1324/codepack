//! Pipeline step 7 ("manifest.json + INDEX.md"): a thin wiring layer over
//! `codepack_reports::write_manifest`/`write_index_md`, ported from legacy
//! `exporter.py`'s manifest-writing call sites.
//!
//! [`write_manifest_and_index`] is designed for **two call sites**, matching legacy's
//! own two manifest writes: this pass calls it once with `archive_result: None`
//! (step 7's own job); a later pass (Group Z, pipeline step 8) calls it again with a
//! real [`codepack_core::ArchiveBuildResult`] once archiving has run, refreshing both
//! files in place. Both writes go through the exact same function, proven by this
//! module's own "call twice" test, so Group Z needs no new wiring beyond supplying a
//! real archive result.

use codepack_core::config::Config;
use codepack_core::{ArchiveBuildResult, CopyStats, ExportPaths, TextDumpStats};
use codepack_diff::DiffSelection;
use codepack_reports::{IndexInput, ManifestInput, write_index_md, write_manifest};

use crate::error::Result;
use crate::timestamp::human_now_utc;

const APP_NAME: &str = "codepack";

/// Writes `manifest.json` and `INDEX.md` from already-produced pipeline data. Pass
/// `archive_result: None` before archiving has run (renders
/// `codepack_reports`'s own documented `{"status":"not_written_yet"}` fallback); pass
/// `Some` after it has, to refresh both files with the real result in place.
///
/// `#[allow(clippy::too_many_arguments)]`: this function's parameter list is a fixed
/// contract mirroring every already-produced pipeline value `codepack_reports`'s own
/// `ManifestInput`/`IndexInput` need; wrapping it in a bespoke struct here would just
/// move the same eight fields one level down without reducing real complexity.
#[allow(clippy::too_many_arguments)]
pub fn write_manifest_and_index(
    paths: &ExportPaths,
    config: &Config,
    copy_stats: &CopyStats,
    text_stats: &TextDumpStats,
    diff_selection: &DiffSelection,
    extra_ignored_dirs: &[String],
    cancelled: bool,
    archive_result: Option<&ArchiveBuildResult>,
) -> Result<()> {
    let app_version = env!("CARGO_PKG_VERSION");
    let generated_at = human_now_utc();

    let manifest_input = ManifestInput {
        app_name: APP_NAME,
        app_version,
        generated_at: generated_at.clone(),
        paths,
        cancelled,
        config,
        extra_ignored_dirs,
        copy_stats,
        text_stats,
        archive_result,
        diff_selection: Some(diff_selection),
    };
    write_manifest(&manifest_input, &paths.manifest_file)?;

    let index_input = IndexInput {
        app_name: APP_NAME,
        app_version,
        generated_at,
        project_name: &paths.project_name,
        config,
        extra_ignored_dirs,
    };
    write_index_md(&index_input, &paths.index_file)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::build_export_paths;

    fn sample_diff_selection() -> DiffSelection {
        DiffSelection {
            mode: "all".to_string(),
            base: "full export".to_string(),
            paths: None,
            files: Vec::new(),
            warning: None,
        }
    }

    #[test]
    fn writes_both_files_with_the_not_written_yet_archive_fallback() {
        let source = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let paths = build_export_paths(source.path(), output.path());
        std::fs::create_dir_all(&paths.staging_dir).unwrap();
        let config = Config::default();
        let copy_stats = CopyStats::default();
        let text_stats = TextDumpStats::default();
        let diff_selection = sample_diff_selection();

        write_manifest_and_index(
            &paths,
            &config,
            &copy_stats,
            &text_stats,
            &diff_selection,
            &[],
            false,
            None,
        )
        .unwrap();

        assert!(paths.manifest_file.is_file());
        assert!(paths.index_file.is_file());
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&paths.manifest_file).unwrap()).unwrap();
        assert_eq!(manifest["archives"]["status"], "not_written_yet");
    }

    #[test]
    fn calling_it_twice_overwrites_cleanly_with_the_real_archive_result() {
        let source = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let paths = build_export_paths(source.path(), output.path());
        std::fs::create_dir_all(&paths.staging_dir).unwrap();
        let config = Config::default();
        let copy_stats = CopyStats::default();
        let text_stats = TextDumpStats::default();
        let diff_selection = sample_diff_selection();

        write_manifest_and_index(
            &paths,
            &config,
            &copy_stats,
            &text_stats,
            &diff_selection,
            &[],
            false,
            None,
        )
        .unwrap();

        let archive_result = ArchiveBuildResult {
            archives: vec![paths.final_zip.clone()],
            output_dir: None,
            split: false,
            file_count: 3,
            skipped_project_files: 0,
            oversized_files: Vec::new(),
        };

        write_manifest_and_index(
            &paths,
            &config,
            &copy_stats,
            &text_stats,
            &diff_selection,
            &[],
            false,
            Some(&archive_result),
        )
        .unwrap();

        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&paths.manifest_file).unwrap()).unwrap();
        assert!(manifest["archives"].get("status").is_none());
        assert_eq!(manifest["archives"]["file_count"], 3);
    }
}
