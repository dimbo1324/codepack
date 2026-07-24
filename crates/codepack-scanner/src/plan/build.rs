//! `build_export_plan()`, ported from legacy `services/export_plan.py`.
//!
//! Safe-export-mode classification is applied through a **caller-supplied predicate**
//! ([`SafetyClassifier`]), following the precedent `codepack-diff` (S4) already set for
//! its own ignored-directory set and text-file predicate: this crate keeps no
//! dependency on `codepack-security`, but the plan it produces is still the complete,
//! legacy-equivalent artifact.
//!
//! This was originally deferred with "safe-export-mode filtering is S3's job", which
//! left `28_export_plan.json` reporting `.env` as `included`/`info` where legacy
//! reports it `excluded`/`critical`. The copy step filtered the file correctly either
//! way, so nothing leaked — but the *plan* misinformed the user, and
//! [`ExportPlan::sensitive_warnings`] (which selects `critical`/`high` entries, and is
//! what a preview UI would warn from) found nothing to warn about. Caught by the
//! golden suite on 2026-07-25.
//!
//! Diff/incremental selection remains the caller's business (`codepack-engine` applies
//! it around this function), matching how legacy threaded `diff_selection` in.

use std::path::Path;

use rayon::prelude::*;

use codepack_core::CancellationToken;

use crate::error::Result;
use crate::ignore::{ExportIgnoreRules, ScanOptions};
use crate::stack;
use crate::walk::{self, IgnoredDirMatcher};

use super::group::classify_group;
use super::timestamp::current_timestamp_utc;
use super::{ExportPlan, PlanSummary, PlannedFile};

/// Decides whether a file must be excluded for export-safety reasons, returning
/// `Some((reason, severity))` when it must. `codepack-engine` supplies
/// `codepack_security::should_skip_file_for_safety` bound to the configured mode;
/// tests that do not care about safety pass [`no_safety_classification`].
pub type SafetyClassifier<'a> = &'a (dyn Fn(&Path) -> Option<(String, String)> + Sync);

/// A [`SafetyClassifier`] that never excludes anything — the exact behavior of legacy's
/// `"full"` safe-export mode.
pub fn no_safety_classification(_relative_path: &Path) -> Option<(String, String)> {
    None
}

pub fn build_export_plan(
    source_root: &Path,
    options: &ScanOptions,
    export_rules: &ExportIgnoreRules,
    safety: SafetyClassifier<'_>,
    cancel: &CancellationToken,
) -> Result<ExportPlan> {
    let stacks = stack::detect_stacks(source_root);
    let stack_dirs = stack::merged_extra_ignored_dirs(&stacks);

    let mut extra_names = options.extra_ignored_dirs.clone();
    extra_names.extend(stack_dirs);
    let matcher = IgnoredDirMatcher::new(extra_names);

    let outcome = walk::walk_project(source_root, &matcher, cancel, |rel_dir| {
        let (skip, reason) = export_rules.should_skip_dir(rel_dir);
        if skip { Some(reason) } else { None }
    })?;

    let cancel_flag = cancel.clone();
    let classified: Vec<PlannedFile> = outcome
        .files
        .par_iter()
        .map(|file| classify_file(file, export_rules, safety, &cancel_flag))
        .collect();
    if cancel.is_cancelled() {
        return Err(crate::error::ScannerError::Cancelled);
    }

    let mut included_files = Vec::with_capacity(classified.len());
    let mut excluded_files = Vec::new();
    for item in classified {
        if item.status == "included" {
            included_files.push(item);
        } else {
            excluded_files.push(item);
        }
    }

    let skipped_dirs: Vec<String> = outcome
        .skipped_dirs
        .iter()
        .map(|dir| match &dir.reason {
            Some(reason) => format!("{} ({reason})", rel_display(&dir.relative_path)),
            None => rel_display(&dir.relative_path),
        })
        .collect();

    let estimated_included_bytes: u64 = included_files.iter().map(|item| item.size).sum();
    let summary = PlanSummary {
        included_count: included_files.len(),
        excluded_count: excluded_files.len(),
        estimated_included_bytes,
        estimated_included_size: codepack_tokens::format_bytes(estimated_included_bytes),
        skipped_dirs_count: skipped_dirs.len(),
    };

    let project_name = source_root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(ExportPlan {
        generated_at: current_timestamp_utc(),
        project_name,
        source_root: source_root.display().to_string(),
        profile: options.export_profile.clone(),
        safe_export_mode: options.safe_export_mode.clone(),
        diff_export_mode: options.diff_export_mode.clone(),
        incremental_enabled: options.incremental_export_enabled,
        included_files,
        excluded_files,
        skipped_dirs,
        warnings: Vec::new(),
        rules: export_rules.to_report(),
        summary,
    })
}

fn classify_file(
    file: &walk::WalkedFile,
    export_rules: &ExportIgnoreRules,
    safety: SafetyClassifier<'_>,
    cancel: &CancellationToken,
) -> PlannedFile {
    let display_path = display_backslash(&file.relative_path);
    let group = classify_group(&file.relative_path).to_string();

    if cancel.is_cancelled() {
        return PlannedFile {
            relative_path: display_path,
            size: file.size,
            status: "excluded".to_string(),
            reason: "cancelled".to_string(),
            severity: "info".to_string(),
            group,
        };
    }

    // Legacy's own order in `export_plan.py`: rule-based exclusion is decided before
    // safety, so a file excluded by an `.exportignore` rule keeps that rule's reason
    // and `"medium"` severity rather than being reported as a credential risk.
    let (skip, reason) = export_rules.should_skip_file(&file.relative_path);
    if skip {
        return PlannedFile {
            relative_path: display_path,
            size: file.size,
            status: "excluded".to_string(),
            reason,
            severity: "medium".to_string(),
            group,
        };
    }

    if let Some((reason, severity)) = safety(&file.relative_path) {
        return PlannedFile {
            relative_path: display_path,
            size: file.size,
            status: "excluded".to_string(),
            reason,
            severity,
            group,
        };
    }

    PlannedFile {
        relative_path: display_path,
        size: file.size,
        status: "included".to_string(),
        reason: String::new(),
        severity: "info".to_string(),
        group,
    }
}

/// Legacy `_normalise_rel`: backslash-joined on every platform, deliberately not
/// OS-dependent (`str(path).replace("/", "\\")`).
fn display_backslash(relative_path: &Path) -> String {
    relative_path.to_string_lossy().replace('/', "\\")
}

/// Legacy `rel_display`: same backslash join, additionally prefixed with `.\` (or
/// rendered as a lone `.` for the project root itself, which `skipped_dirs` never
/// actually contains in practice).
fn rel_display(relative_path: &Path) -> String {
    if relative_path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        format!(".\\{}", display_backslash(relative_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn cancel_never() -> CancellationToken {
        CancellationToken::new()
    }

    #[test]
    fn empty_project_has_no_files() {
        let dir = tempfile::tempdir().unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        assert_eq!(plan.summary.included_count, 0);
        assert_eq!(plan.summary.excluded_count, 0);
    }

    #[test]
    fn single_python_file_is_included() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.py"), "print(1)").unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        assert_eq!(plan.included_files.len(), 1);
        assert_eq!(plan.included_files[0].relative_path, "main.py");
        assert_eq!(plan.included_files[0].group, "python_source");
    }

    #[test]
    fn nested_directories_are_included() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("src/core")).unwrap();
        fs::write(dir.path().join("src/core/utils.py"), "x").unwrap();
        fs::write(dir.path().join("src/core/models.py"), "x").unwrap();
        fs::write(dir.path().join("README.md"), "x").unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        assert_eq!(plan.included_files.len(), 3);
    }

    #[test]
    fn exportignore_excludes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.py"), "x").unwrap();
        fs::write(dir.path().join("debug.log"), "x").unwrap();
        fs::write(dir.path().join(".exportignore"), "*.log\n").unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        let included: Vec<&str> = plan
            .included_files
            .iter()
            .map(|f| f.relative_path.as_str())
            .collect();
        assert!(included.contains(&"main.py"));
        assert!(!included.contains(&"debug.log"));
        assert_eq!(plan.excluded_files.len(), 1);
        assert_eq!(plan.excluded_files[0].severity, "medium");
    }

    #[test]
    fn config_extra_ignored_dir_prunes_files_beneath_it() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.py"), "x").unwrap();
        fs::create_dir_all(dir.path().join("vendor_lib")).unwrap();
        fs::write(dir.path().join("vendor_lib/dep.py"), "x").unwrap();
        let options = ScanOptions {
            extra_ignored_dirs: vec!["vendor_lib".to_string()],
            ..ScanOptions::default()
        };
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        assert_eq!(plan.included_files.len(), 1);
        assert!(
            plan.skipped_dirs.iter().any(|d| d == ".\\vendor_lib"),
            "skipped_dirs = {:?}",
            plan.skipped_dirs
        );
    }

    #[test]
    fn always_include_overrides_exportignore() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".exportignore"), "*.log\n").unwrap();
        fs::write(dir.path().join("app.log"), "x").unwrap();
        fs::write(dir.path().join("main.py"), "x").unwrap();
        let options = ScanOptions {
            always_include_files: vec!["app.log".to_string()],
            ..ScanOptions::default()
        };
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        let included: Vec<&str> = plan
            .included_files
            .iter()
            .map(|f| f.relative_path.as_str())
            .collect();
        assert!(included.contains(&"app.log"));
    }

    #[test]
    fn estimated_bytes_matches_included_file_sizes() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("file.py"), "x".repeat(1000)).unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        let total: u64 = plan.included_files.iter().map(|f| f.size).sum();
        assert_eq!(total, plan.summary.estimated_included_bytes);
        assert_eq!(plan.summary.estimated_included_bytes, 1000);
    }

    #[test]
    fn empty_file_is_included_with_zero_size() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("empty.py"), "").unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        assert_eq!(plan.included_files.len(), 1);
        assert_eq!(plan.summary.estimated_included_bytes, 0);
    }

    #[test]
    fn project_name_comes_from_the_source_root_directory_name() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.py"), "x").unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        assert_eq!(
            plan.project_name,
            dir.path().file_name().unwrap().to_string_lossy()
        );
    }

    #[test]
    fn always_include_does_not_rescue_files_inside_a_base_ignored_directory() {
        // Precedence test: always-include beats `.exportignore`/custom exclusion, but
        // never beats base/stack directory pruning — a file under `node_modules` can
        // never be rescued, matching legacy's `should_ignore_dir(...) or is_symlink`
        // check that runs *before* the ignore-rules engine is even consulted.
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/pkg.js"), "x").unwrap();
        let options = ScanOptions {
            always_include_files: vec!["node_modules/pkg.js".to_string()],
            ..ScanOptions::default()
        };
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        assert!(plan.included_files.is_empty());
        assert!(plan.excluded_files.is_empty());
        assert_eq!(plan.skipped_dirs, vec![".\\node_modules".to_string()]);
    }

    #[test]
    fn stack_detected_directories_are_pruned_even_without_config_entries() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::create_dir_all(dir.path().join("target")).unwrap();
        fs::write(dir.path().join("target/build.log"), "x").unwrap();
        fs::write(dir.path().join("src_main.rs"), "fn main() {}").unwrap();
        let options = ScanOptions::default();
        let rules = ExportIgnoreRules::from_project_and_config(dir.path(), &options);
        let plan = build_export_plan(
            dir.path(),
            &options,
            &rules,
            &no_safety_classification,
            &cancel_never(),
        )
        .unwrap();
        let included: Vec<&str> = plan
            .included_files
            .iter()
            .map(|f| f.relative_path.as_str())
            .collect();
        assert!(!included.iter().any(|p| p.contains("target")));
    }
}
