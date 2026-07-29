//! `ExportPlan`: the pre-copy inventory of every file the export would include or
//! exclude, ported from legacy `services/export_plan.py`. **S2-scoped**: no
//! safe-export-mode filtering (S3) and no diff/incremental filtering (S4) — see the
//! crate-level scope boundary in `lib.rs`.
//!
//! Split into a directory module (base+stack+custom-rule logic, group classification,
//! and JSON/Markdown rendering do not comfortably fit under the 600-line guideline in
//! one file — `.ai/project/12-domain-rules.md`), while keeping the flat
//! `codepack_scanner::plan::*` surface legacy's single `export_plan.py` module had.

mod build;
mod group;
mod render;

pub use build::{SafetyClassifier, build_export_plan, no_safety_classification};
pub use render::write_export_plan_files;

use serde::{Deserialize, Serialize};

use crate::ignore::RulesReport;

/// One file's disposition in the plan. Field order is a documented contract
/// (invariant I5, `docs/__arch__/BLUEPRINT.md` artifact formats): `relative_path`, `size`,
/// `status`, `reason`, `severity`, `group`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedFile {
    pub relative_path: String,
    pub size: u64,
    pub status: String,
    #[serde(default)]
    pub reason: String,
    pub severity: String,
    pub group: String,
}

/// Top-level export plan. Field order is a documented contract (invariant I5):
/// `generated_at`, `project_name`, `source_root`, `profile`, `safe_export_mode`,
/// `diff_export_mode`, `incremental_enabled`, `included_files`, `excluded_files`,
/// `skipped_dirs`, `warnings`, `rules`, `summary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPlan {
    pub generated_at: String,
    pub project_name: String,
    pub source_root: String,
    pub profile: String,
    pub safe_export_mode: String,
    pub diff_export_mode: String,
    pub incremental_enabled: bool,
    pub included_files: Vec<PlannedFile>,
    pub excluded_files: Vec<PlannedFile>,
    pub skipped_dirs: Vec<String>,
    pub warnings: Vec<String>,
    pub rules: RulesReport,
    pub summary: PlanSummary,
}

/// Field-for-field with legacy's own `write_export_plan_files` summary block.
///
/// `estimated_included_size` and `skipped_dirs_count` were originally omitted with the
/// reasoning "`codepack-core` has no byte-formatter yet (that lands in S6)" and
/// "already available as `skipped_dirs.len()`". S6 shipped
/// [`codepack_tokens::format_bytes`] and the reason expired, but the fields were never
/// restored — a real break of invariant I5 (`28_export_plan.json` is a named artifact
/// contract), found by the golden suite on 2026-07-25. Restoring them needs no
/// `schema_version` bump: it returns the format to the contract it was never supposed
/// to leave.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub included_count: usize,
    pub excluded_count: usize,
    pub estimated_included_bytes: u64,
    pub estimated_included_size: String,
    pub skipped_dirs_count: usize,
}

impl ExportPlan {
    pub fn sensitive_warnings(&self) -> Vec<&PlannedFile> {
        self.excluded_files
            .iter()
            .filter(|item| item.severity == "critical" || item.severity == "high")
            .collect()
    }

    pub fn large_files(&self) -> Vec<&PlannedFile> {
        self.included_files
            .iter()
            .chain(self.excluded_files.iter())
            .filter(|item| item.size >= LARGE_FILE_THRESHOLD_BYTES)
            .collect()
    }
}

/// Size at which a file is worth calling out to the user before the export runs.
///
/// Legacy's own threshold. Not a limit — nothing is excluded for being large — only the
/// point at which a preview is expected to mention the file, since a single file this
/// size dominates both the archive and any token budget.
const LARGE_FILE_THRESHOLD_BYTES: u64 = 100 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ignore::RulesReport;

    fn planned(relative_path: &str, size: u64, status: &str, severity: &str) -> PlannedFile {
        PlannedFile {
            relative_path: relative_path.to_string(),
            size,
            status: status.to_string(),
            reason: String::new(),
            severity: severity.to_string(),
            group: "other".to_string(),
        }
    }

    fn plan_of(included: Vec<PlannedFile>, excluded: Vec<PlannedFile>) -> ExportPlan {
        ExportPlan {
            generated_at: "1970-01-01 00:00:00 UTC".to_string(),
            project_name: "demo".to_string(),
            source_root: "/demo".to_string(),
            profile: "full".to_string(),
            safe_export_mode: "safe".to_string(),
            diff_export_mode: "all".to_string(),
            incremental_enabled: false,
            included_files: included,
            excluded_files: excluded,
            skipped_dirs: Vec::new(),
            warnings: Vec::new(),
            rules: RulesReport {
                source_file: None,
                loaded_rules: Vec::new(),
                excluded_dirs: Vec::new(),
                excluded_files: Vec::new(),
                excluded_extensions: Vec::new(),
                always_include_files: Vec::new(),
                always_include_dirs: Vec::new(),
            },
            summary: PlanSummary {
                included_count: 0,
                excluded_count: 0,
                estimated_included_bytes: 0,
                estimated_included_size: "0 B".to_string(),
                skipped_dirs_count: 0,
            },
        }
    }

    #[test]
    fn sensitive_warnings_selects_only_the_two_risk_severities() {
        // This is what a preview UI warns from, so a severity slipping out of the set
        // would silently stop warning the user about an excluded credential.
        let plan = plan_of(
            vec![planned("main.rs", 1, "included", "info")],
            vec![
                planned(".env", 1, "excluded", "critical"),
                planned("id_rsa", 1, "excluded", "high"),
                planned("debug.log", 1, "excluded", "medium"),
                planned("notes.txt", 1, "excluded", "info"),
            ],
        );

        let warned: Vec<&str> = plan
            .sensitive_warnings()
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect();
        assert_eq!(warned, vec![".env", "id_rsa"]);
    }

    #[test]
    fn sensitive_warnings_never_reports_an_included_file() {
        // Only exclusions are warnings: an included file is not a risk the user needs
        // to act on before exporting.
        let plan = plan_of(
            vec![planned("kept.pem", 1, "included", "critical")],
            Vec::new(),
        );
        assert!(plan.sensitive_warnings().is_empty());
    }

    #[test]
    fn large_files_spans_both_lists_and_respects_the_threshold() {
        let plan = plan_of(
            vec![
                planned("big.bin", LARGE_FILE_THRESHOLD_BYTES, "included", "info"),
                planned("small.rs", 10, "included", "info"),
            ],
            vec![planned(
                "huge.zip",
                LARGE_FILE_THRESHOLD_BYTES * 3,
                "excluded",
                "high",
            )],
        );

        let large: Vec<&str> = plan
            .large_files()
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect();
        assert_eq!(large, vec!["big.bin", "huge.zip"]);
    }

    #[test]
    fn the_large_file_threshold_is_inclusive_at_its_boundary() {
        let plan = plan_of(
            vec![
                planned("at.bin", LARGE_FILE_THRESHOLD_BYTES, "included", "info"),
                planned(
                    "under.bin",
                    LARGE_FILE_THRESHOLD_BYTES - 1,
                    "included",
                    "info",
                ),
            ],
            Vec::new(),
        );
        let large: Vec<&str> = plan
            .large_files()
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect();
        assert_eq!(large, vec!["at.bin"]);
    }

    #[test]
    fn both_helpers_are_empty_on_an_empty_plan() {
        let plan = plan_of(Vec::new(), Vec::new());
        assert!(plan.sensitive_warnings().is_empty());
        assert!(plan.large_files().is_empty());
    }
}
