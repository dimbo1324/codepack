//! Shared result and report types produced by the export pipeline (BLUEPRINT §A.2).
//!
//! These are plain data: construction is the job of the crate that owns each pipeline
//! step (`codepack-scanner`, `codepack-archive`, `codepack-engine`, ...).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportPaths {
    pub desktop: PathBuf,
    pub source_root: PathBuf,
    pub project_name: String,
    pub bundle_name: String,
    pub staging_dir: PathBuf,
    pub final_zip: PathBuf,
    pub archive_set_dir: PathBuf,
    pub project_dir: PathBuf,
    pub reports_dir: PathBuf,
    pub insights_dir: PathBuf,
    pub manifest_file: PathBuf,
    pub project_profile_file: PathBuf,
    pub index_file: PathBuf,
    pub structure_report: PathBuf,
    pub git_report: PathBuf,
    pub text_dump: PathBuf,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyStats {
    pub dirs_created: u32,
    pub files_copied: u32,
    pub dirs_skipped: u32,
    pub files_skipped: u32,
    pub files_skipped_by_safety: u32,
    pub files_skipped_by_diff: u32,
    pub symlinks_skipped: u32,
    pub errors: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextDumpStats {
    pub scanned: u32,
    pub written: u32,
    pub skipped_binary: u32,
    pub skipped_large: u32,
    pub skipped_decode: u32,
    pub skipped_not_text: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskPreviewItem {
    pub path: String,
    pub reason: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default = "default_severity")]
    pub severity: String,
}

fn default_severity() -> String {
    "medium".to_string()
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskPreviewReport {
    pub scanned_files: u32,
    pub scanned_dirs: u32,
    #[serde(default)]
    pub ignored_dirs: u32,
    pub sensitive_files: Vec<RiskPreviewItem>,
    pub large_files: Vec<RiskPreviewItem>,
    pub archive_or_dump_files: Vec<RiskPreviewItem>,
    #[serde(default)]
    pub estimated_selected_bytes: u64,
    #[serde(default)]
    pub diff_limited: bool,
    #[serde(default)]
    pub diff_file_count: Option<u32>,
    #[serde(default)]
    pub git_warning: Option<String>,
}

impl RiskPreviewReport {
    pub fn has_warnings(&self) -> bool {
        !self.sensitive_files.is_empty()
            || !self.large_files.is_empty()
            || !self.archive_or_dump_files.is_empty()
            || self.git_warning.is_some()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveBuildResult {
    #[serde(default)]
    pub archives: Vec<PathBuf>,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub split: bool,
    pub file_count: u32,
    #[serde(default)]
    pub skipped_project_files: u32,
    #[serde(default)]
    pub oversized_files: Vec<String>,
}

impl ArchiveBuildResult {
    /// The single artifact a caller should point the user at: the split-set
    /// directory when the build split, otherwise the first (only) archive file.
    pub fn primary_result(&self) -> Option<&std::path::Path> {
        if self.split
            && let Some(dir) = &self.output_dir
        {
            return Some(dir.as_path());
        }
        self.archives
            .first()
            .map(PathBuf::as_path)
            .or(self.output_dir.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_preview_report_has_warnings_when_any_list_is_non_empty() {
        let mut report = RiskPreviewReport::default();
        assert!(!report.has_warnings());

        report.sensitive_files.push(RiskPreviewItem {
            path: "secrets/.env".to_string(),
            reason: "sensitive file".to_string(),
            size: 12,
            severity: "high".to_string(),
        });
        assert!(report.has_warnings());
    }

    #[test]
    fn risk_preview_report_has_warnings_when_git_warning_is_set() {
        let report = RiskPreviewReport {
            git_warning: Some("detached HEAD".to_string()),
            ..Default::default()
        };
        assert!(report.has_warnings());
    }

    #[test]
    fn archive_build_result_primary_result_prefers_output_dir_when_split() {
        let result = ArchiveBuildResult {
            split: true,
            output_dir: Some(PathBuf::from("/out/archive_set")),
            archives: vec![PathBuf::from("/out/archive_set/part_001.zip")],
            ..Default::default()
        };
        assert_eq!(
            result.primary_result(),
            Some(std::path::Path::new("/out/archive_set"))
        );
    }

    #[test]
    fn archive_build_result_primary_result_falls_back_to_first_archive() {
        let result = ArchiveBuildResult {
            split: false,
            archives: vec![PathBuf::from("/out/export.zip")],
            ..Default::default()
        };
        assert_eq!(
            result.primary_result(),
            Some(std::path::Path::new("/out/export.zip"))
        );
    }

    #[test]
    fn archive_build_result_primary_result_none_when_nothing_built() {
        let result = ArchiveBuildResult::default();
        assert_eq!(result.primary_result(), None);
    }
}
