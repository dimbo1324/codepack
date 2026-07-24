//! [`full_report_catalog`]: the single source of truth for `manifest.json`'s
//! `layout.reports` list (`crate::metadata::write_manifest`) and `INDEX.md`'s
//! "## Reports" section (`crate::metadata::write_index_md`) — the Rust analogue of
//! legacy `constants.py::REPORT_DESCRIPTIONS`.
//!
//! Unlike legacy, this is not a flat static tuple: each numbered-report entry pulls
//! its filename/description straight from the owning [`crate::plugin::ReportJob`] so
//! the two never drift apart. Three entries (`27_archive_plan.md`,
//! `28_export_plan.md`, `29_export_comparison_report.md`) are fixed strings rather
//! than a `ReportJob` lookup: they are real parts of the full export bundle, but are
//! produced by other crates (`codepack-archive` stage S8, `codepack-scanner` stage
//! S2, `codepack-diff` stage S4 respectively) — this crate lists them for a complete,
//! accurate manifest/index, exactly as legacy's own `REPORT_DESCRIPTIONS` lists
//! reports produced outside `reports/insights/` too.

use crate::reports::{ai_context_folder, ai_context_pack, ai_prompts, dashboard, runbook};
use crate::reports::{
    api_surface, architecture, architecture_map, backend, code_metrics, code_quality, config,
    dependencies, dependency_graph, dependency_intelligence, docker, file_statistics, frontend,
    git_deep, git_timeline, key_files, large_files, project_health, refactoring, routes_and_pages,
    scripts, security_scan, summary, todo_fixme,
};

pub const PROJECT_PROFILE_DESCRIPTION: &str =
    "Machine-readable project passport: stack, commands, entrypoints, capabilities, risk level.";

/// The full catalog, in the numeric/legacy order from BLUEPRINT §A.7. `(filename,
/// description)` pairs; `filename` also doubles as the relative link target used by
/// `INDEX.md` and the dashboard.
pub fn full_report_catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        ("PROJECT_PROFILE.json", PROJECT_PROFILE_DESCRIPTION),
        (summary::JOB.filename, summary::JOB.description),
        (
            file_statistics::JOB.filename,
            file_statistics::JOB.description,
        ),
        (dependencies::JOB.filename, dependencies::JOB.description),
        (scripts::JOB.filename, scripts::JOB.description),
        (git_deep::JOB.filename, git_deep::JOB.description),
        (security_scan::JOB.filename, security_scan::JOB.description),
        (
            "06_security_scan.json",
            "Machine-readable security findings JSON.",
        ),
        (
            "06_security_scan.sarif",
            "SARIF security findings for code-review/security tooling.",
        ),
        (todo_fixme::JOB.filename, todo_fixme::JOB.description),
        (code_metrics::JOB.filename, code_metrics::JOB.description),
        (config::JOB.filename, config::JOB.description),
        (docker::JOB.filename, docker::JOB.description),
        (
            routes_and_pages::JOB.filename,
            routes_and_pages::JOB.description,
        ),
        (
            ai_context_pack::JOB.filename,
            ai_context_pack::JOB.description,
        ),
        (runbook::JOB.filename, runbook::JOB.description),
        (
            dependency_graph::JOB.filename,
            dependency_graph::JOB.description,
        ),
        (
            "14_dependency_graph.mmd",
            "Mermaid dependency graph for visual architecture review.",
        ),
        (architecture::JOB.filename, architecture::JOB.description),
        (key_files::JOB.filename, key_files::JOB.description),
        (code_quality::JOB.filename, code_quality::JOB.description),
        (api_surface::JOB.filename, api_surface::JOB.description),
        (frontend::JOB.filename, frontend::JOB.description),
        (backend::JOB.filename, backend::JOB.description),
        (git_timeline::JOB.filename, git_timeline::JOB.description),
        (
            project_health::JOB.filename,
            project_health::JOB.description,
        ),
        (refactoring::JOB.filename, refactoring::JOB.description),
        (
            architecture_map::JOB.filename,
            architecture_map::JOB.description,
        ),
        (large_files::JOB.filename, large_files::JOB.description),
        (
            dependency_intelligence::JOB.filename,
            dependency_intelligence::JOB.description,
        ),
        (
            "27_archive_plan.md",
            "Archive plan: single ZIP or logical split parts (produced by codepack-archive, stage S8).",
        ),
        (
            "28_export_plan.md",
            "Pre-copy export plan: included/excluded files, warnings, large files, selected size (produced by codepack-scanner, stage S2).",
        ),
        (
            "29_export_comparison_report.md",
            "Incremental export comparison against the previous successful baseline (produced by codepack-diff, stage S4).",
        ),
        (dashboard::JOB.filename, dashboard::JOB.description),
        ("AI_CONTEXT/", ai_context_folder::JOB.description),
        ("AI_PROMPTS/", ai_prompts::JOB.description),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_has_a_non_empty_filename_and_description() {
        for (filename, description) in full_report_catalog() {
            assert!(!filename.is_empty());
            assert!(!description.is_empty());
        }
    }

    #[test]
    fn contains_every_numbered_report_and_the_three_out_of_crate_placeholders() {
        let catalog = full_report_catalog();
        let names: Vec<&str> = catalog.iter().map(|(name, _)| *name).collect();
        for expected in [
            "PROJECT_PROFILE.json",
            "01_summary.txt",
            "06_security_scan.json",
            "06_security_scan.sarif",
            "14_dependency_graph.mmd",
            "26_dependency_intelligence.md",
            "27_archive_plan.md",
            "28_export_plan.md",
            "29_export_comparison_report.md",
            "REPORT_DASHBOARD.html",
            "AI_CONTEXT/",
            "AI_PROMPTS/",
        ] {
            assert!(
                names.contains(&expected),
                "missing catalog entry: {expected}"
            );
        }
    }
}
