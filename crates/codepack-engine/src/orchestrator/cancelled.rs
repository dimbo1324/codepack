//! The synthetic outcome for a run cancelled before pipeline step 1 began.
//!
//! Split out of `orchestrator.rs` on 2026-07-27 (finding 6, audit). Kept together
//! with `finding_kind_label` because both are pure value construction with no
//! pipeline logic of their own.

use codepack_core::ExportPaths;
use codepack_core::config::Config;
use codepack_security::FindingKind;

use crate::ignored_dirs::ignored_dir_names_for;
use crate::plan::PlanOutcome;
use crate::timestamp::human_now_utc;

/// A synthetic "nothing was planned or copied" [`PlanOutcome`] for the one case
/// [`run_export`] resolves without calling step 1 at all: a token already cancelled
/// before the pipeline begins. See `run_export`'s own inline comment at its call site
/// for why this exists instead of calling [`run_export_plan`] and letting it hard-fail.
pub(super) fn cancelled_before_planning_outcome(
    paths: &ExportPaths,
    config: &Config,
) -> PlanOutcome {
    let ignored_dir_names = ignored_dir_names_for(&paths.source_root, config);

    let export_plan = codepack_scanner::ExportPlan {
        generated_at: human_now_utc(),
        project_name: paths.project_name.clone(),
        source_root: paths.source_root.display().to_string(),
        profile: config.normalized_export_profile().to_string(),
        safe_export_mode: config.normalized_safe_export_mode().to_string(),
        diff_export_mode: config.normalized_diff_export_mode().to_string(),
        incremental_enabled: config.incremental_export_enabled,
        included_files: Vec::new(),
        excluded_files: Vec::new(),
        skipped_dirs: Vec::new(),
        warnings: vec!["export cancelled before pipeline step 1 began".to_string()],
        rules: codepack_scanner::RulesReport {
            source_file: None,
            loaded_rules: Vec::new(),
            excluded_dirs: Vec::new(),
            excluded_files: Vec::new(),
            excluded_extensions: Vec::new(),
            always_include_files: Vec::new(),
            always_include_dirs: Vec::new(),
        },
        summary: codepack_scanner::plan::PlanSummary {
            included_count: 0,
            excluded_count: 0,
            estimated_included_bytes: 0,
            estimated_included_size: codepack_tokens::format_bytes(0),
            skipped_dirs_count: 0,
        },
    };

    let diff_selection = codepack_diff::DiffSelection {
        mode: "cancelled".to_string(),
        base: "cancelled before planning began".to_string(),
        paths: None,
        files: Vec::new(),
        warning: Some("export cancelled before pipeline step 1 began".to_string()),
    };

    PlanOutcome {
        export_plan,
        diff_selection,
        ignored_dir_names,
        include_relative_paths: None,
        dropped_by_budget: 0,
    }
}

pub(super) fn finding_kind_label(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::SensitiveFile => "sensitive_file",
        FindingKind::PotentialSecret => "potential_secret",
        FindingKind::RiskyCode => "risky_code",
    }
}
