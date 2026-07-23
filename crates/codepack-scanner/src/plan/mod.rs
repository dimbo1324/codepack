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
mod timestamp;

pub use build::build_export_plan;
pub use render::write_export_plan_files;

use serde::{Deserialize, Serialize};

use crate::ignore::RulesReport;

/// One file's disposition in the plan. Field order is a documented contract
/// (invariant I5, `BLUEPRINT.md` artifact formats): `relative_path`, `size`,
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

/// `estimated_included_size` (a formatted-bytes string in legacy) is intentionally
/// omitted: `codepack-core` has no byte-formatter yet (that lands in S6, BLUEPRINT
/// §B.2/§E). `skipped_dirs_count` is likewise omitted — it is already available as
/// `skipped_dirs.len()` at the plan's top level, so this stage does not duplicate it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub included_count: usize,
    pub excluded_count: usize,
    pub estimated_included_bytes: u64,
}

impl ExportPlan {
    pub fn sensitive_warnings(&self) -> Vec<&PlannedFile> {
        self.excluded_files
            .iter()
            .filter(|item| item.severity == "critical" || item.severity == "high")
            .collect()
    }

    pub fn large_files(&self) -> Vec<&PlannedFile> {
        const HUNDRED_MB: u64 = 100 * 1024 * 1024;
        self.included_files
            .iter()
            .chain(self.excluded_files.iter())
            .filter(|item| item.size >= HUNDRED_MB)
            .collect()
    }
}
