//! "Fit to budget" (BLUEPRINT §B.3): keep the export under a token budget by dropping
//! the least valuable files instead of truncating arbitrarily.
//!
//! `codepack-tokens` shipped [`codepack_tokens::fit_to_budget`] in S6 with no caller at
//! all, so ROADMAP §6 claimed the capability while the pipeline never invoked it. This
//! module is that caller.
//!
//! Two design points worth stating, because both were live choices:
//!
//! * **Importance comes from the ranking that already exists.** BLUEPRINT §B.3 asks for
//!   prioritization "по уже существующему ранжированию (`16_key_files_report`)", so this
//!   calls [`codepack_reports::reports::key_files::importance_ranking`] rather than
//!   inventing a second scorer that would inevitably disagree with the report the user
//!   is reading. A file the ranking has no opinion about gets importance `1` — low, but
//!   never zero, so it still competes on token cost instead of being silently unranked.
//! * **Tokens are estimated with the fallback formula.** [`codepack_tokens`] offers a
//!   refined estimate too, but the budget must agree with the token counts the rest of
//!   the pipeline reports (history, `01_summary`), and those use the fallback. Invariant
//!   I4's rule — one estimate never silently standing in for the other — applies here.
//!
//! Files the user pinned (`always_include_files`/`always_include_dirs`, or a per-file
//! override) are never dropped: an explicit instruction outranks a heuristic. They still
//! consume budget, so pinning more than the budget allows simply means the budget is
//! exceeded — silently discarding what the user demanded would be worse.
//!
//! The whole pass is skipped unless `Config::token_budget` is non-zero, which is the
//! default, so an export that does not ask for a budget pays nothing for this module.

use std::collections::BTreeMap;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_reports::context::{Inventory, ReportContext};
use codepack_scanner::{ExportIgnoreRules, ExportPlan};
use codepack_tokens::{BudgetCandidate, estimate_tokens_fallback};

use crate::relpath::to_relative_path;

use std::path::Path;

/// Importance given to a file the ranking scored zero or never scored. Deliberately
/// non-zero: `fit_to_budget` ranks by `importance / tokens`, and a zero would park the
/// file behind every other candidate regardless of how cheap it is.
const UNRANKED_IMPORTANCE: f64 = 1.0;

/// Rewrites `plan` so its `included_files` fit within `config.token_budget`, moving
/// everything that did not fit into `excluded_files`. A zero budget is a no-op.
///
/// Returns the number of files dropped, for the caller's own progress log.
pub(crate) fn apply_token_budget(
    plan: &mut ExportPlan,
    source_root: &Path,
    config: &Config,
    rules: &ExportIgnoreRules,
    cancel: &CancellationToken,
) -> usize {
    if config.token_budget == 0 || plan.included_files.is_empty() {
        return 0;
    }
    // `importance_ranking` reads the text of every planned file to build the import
    // graph, which on a large project is not a quick pass. Checking here keeps a
    // cancelled export from starting work it will throw away; leaving the plan untouched
    // is the right degradation, since the export is stopping anyway.
    if cancel.is_cancelled() {
        return 0;
    }

    let ranking = importance_of_planned_files(plan, source_root, config, cancel);
    if cancel.is_cancelled() {
        return 0;
    }

    let candidates: Vec<BudgetCandidate> = plan
        .included_files
        .iter()
        .map(|file| BudgetCandidate {
            id: file.relative_path.clone(),
            tokens: estimate_tokens_fallback(file.size),
            importance: ranking
                .get(&file.relative_path)
                .map(|score| *score as f64)
                .unwrap_or(UNRANKED_IMPORTANCE),
        })
        .collect();

    let selection = codepack_tokens::fit_to_budget(&candidates, config.token_budget);

    let kept: std::collections::HashSet<&str> =
        selection.included.iter().map(String::as_str).collect();
    let is_pinned = |relative_path: &str| rules.is_pinned_file(&to_relative_path(relative_path));
    let mut included = Vec::with_capacity(kept.len());
    let mut removed = 0usize;
    for file in std::mem::take(&mut plan.included_files) {
        if kept.contains(file.relative_path.as_str()) || is_pinned(&file.relative_path) {
            included.push(file);
        } else {
            let mut dropped = file;
            dropped.status = "excluded".to_string();
            dropped.reason = "does not fit the configured token budget".to_string();
            dropped.severity = "info".to_string();
            plan.excluded_files.push(dropped);
            removed += 1;
        }
    }

    plan.included_files = included;
    refresh_summary(plan);
    removed
}

/// Builds a [`ReportContext`] over the *source* tree just far enough to reuse the
/// key-files ranking. `scan`/`diff` are `None`: neither contributes to the score.
fn importance_of_planned_files(
    plan: &ExportPlan,
    source_root: &Path,
    config: &Config,
    cancel: &CancellationToken,
) -> BTreeMap<String, i64> {
    let inventory = Inventory::from_plan(plan);
    let ctx = ReportContext {
        source_root: source_root.to_path_buf(),
        staging_root: source_root.to_path_buf(),
        inventory: &inventory,
        plan,
        scan: None,
        diff: None,
        config,
        cancel,
        profile: config.normalized_export_profile(),
    };
    codepack_reports::reports::key_files::importance_ranking(&ctx)
}

fn refresh_summary(plan: &mut ExportPlan) {
    let estimated_included_bytes: u64 = plan.included_files.iter().map(|file| file.size).sum();
    plan.summary.included_count = plan.included_files.len();
    plan.summary.excluded_count = plan.excluded_files.len();
    plan.summary.estimated_included_bytes = estimated_included_bytes;
    plan.summary.estimated_included_size = codepack_tokens::format_bytes(estimated_included_bytes);
    plan.summary.skipped_dirs_count = plan.skipped_dirs.len();
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_scanner::{ScanOptions, build_export_plan, plan::no_safety_classification};

    /// A project whose files differ enough in size that the budget has a real choice to
    /// make. Sizes are chosen so `estimate_tokens_fallback` (bytes / 3.5) gives each file
    /// a clearly different cost.
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.py"), "x".repeat(70)).unwrap();
        std::fs::write(dir.path().join("helper.py"), "y".repeat(350)).unwrap();
        std::fs::write(dir.path().join("notes.md"), "z".repeat(700)).unwrap();
        dir
    }

    fn plan_for(dir: &Path, options: &ScanOptions) -> (ExportPlan, ExportIgnoreRules) {
        let rules = ExportIgnoreRules::from_project_and_config(dir, options);
        let plan = build_export_plan(
            dir,
            options,
            &rules,
            &no_safety_classification,
            &CancellationToken::new(),
        )
        .unwrap();
        (plan, rules)
    }

    fn config_with_budget(token_budget: u64) -> Config {
        Config {
            token_budget,
            ..Config::default()
        }
    }

    #[test]
    fn a_zero_budget_is_a_no_op() {
        // The default, so an export that does not ask for a budget must pay nothing.
        let dir = fixture();
        let options = ScanOptions::default();
        let (mut plan, rules) = plan_for(dir.path(), &options);
        let before = plan.included_files.len();

        let removed = apply_token_budget(
            &mut plan,
            dir.path(),
            &config_with_budget(0),
            &rules,
            &CancellationToken::new(),
        );

        assert_eq!(removed, 0);
        assert_eq!(plan.included_files.len(), before);
        assert!(plan.excluded_files.is_empty());
    }

    #[test]
    fn a_budget_large_enough_for_everything_drops_nothing() {
        let dir = fixture();
        let options = ScanOptions::default();
        let (mut plan, rules) = plan_for(dir.path(), &options);
        let before = plan.included_files.len();

        let removed = apply_token_budget(
            &mut plan,
            dir.path(),
            &config_with_budget(1_000_000),
            &rules,
            &CancellationToken::new(),
        );

        assert_eq!(removed, 0);
        assert_eq!(plan.included_files.len(), before);
    }

    #[test]
    fn a_tight_budget_drops_files_and_records_why() {
        let dir = fixture();
        let options = ScanOptions::default();
        let (mut plan, rules) = plan_for(dir.path(), &options);

        // Room for the smallest file only: 70 bytes is 20 tokens, the next is 100.
        let removed = apply_token_budget(
            &mut plan,
            dir.path(),
            &config_with_budget(25),
            &rules,
            &CancellationToken::new(),
        );

        assert!(removed > 0, "a 25-token budget cannot hold every file");
        assert_eq!(plan.excluded_files.len(), removed);
        for dropped in &plan.excluded_files {
            assert_eq!(dropped.status, "excluded");
            assert_eq!(dropped.reason, "does not fit the configured token budget");
            // `info`, not a risk severity: dropping for size is not a security signal,
            // and `ExportPlan::sensitive_warnings` selects on critical/high.
            assert_eq!(dropped.severity, "info");
        }
    }

    #[test]
    fn a_pinned_file_survives_a_budget_that_would_otherwise_drop_it() {
        // An explicit instruction outranks a heuristic: always_include_files is the user
        // saying "this one matters", so the budget must not overrule it.
        let dir = fixture();
        let options = ScanOptions {
            always_include_files: vec!["notes.md".to_string()],
            ..ScanOptions::default()
        };
        let (mut plan, rules) = plan_for(dir.path(), &options);

        apply_token_budget(
            &mut plan,
            dir.path(),
            // Far too small for the 700-byte notes.md on its own merits.
            &config_with_budget(5),
            &rules,
            &CancellationToken::new(),
        );

        assert!(
            plan.included_files
                .iter()
                .any(|file| file.relative_path == "notes.md"),
            "pinned file was dropped: {:?}",
            plan.included_files
                .iter()
                .map(|f| &f.relative_path)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_summary_is_refreshed_to_match_what_survived() {
        // A stale summary would misreport the export's size in `28_export_plan.json`.
        let dir = fixture();
        let options = ScanOptions::default();
        let (mut plan, rules) = plan_for(dir.path(), &options);

        apply_token_budget(
            &mut plan,
            dir.path(),
            &config_with_budget(25),
            &rules,
            &CancellationToken::new(),
        );

        let expected_bytes: u64 = plan.included_files.iter().map(|file| file.size).sum();
        assert_eq!(plan.summary.included_count, plan.included_files.len());
        assert_eq!(plan.summary.excluded_count, plan.excluded_files.len());
        assert_eq!(plan.summary.estimated_included_bytes, expected_bytes);
        assert_eq!(
            plan.summary.estimated_included_size,
            codepack_tokens::format_bytes(expected_bytes)
        );
    }

    #[test]
    fn a_cancelled_export_leaves_the_plan_untouched() {
        // Degrading to "no change" is right: the export is stopping anyway, and the
        // ranking pass reads every planned file, which is work worth skipping.
        let dir = fixture();
        let options = ScanOptions::default();
        let (mut plan, rules) = plan_for(dir.path(), &options);
        let before = plan.included_files.len();

        let cancel = CancellationToken::new();
        cancel.cancel();
        let removed = apply_token_budget(
            &mut plan,
            dir.path(),
            &config_with_budget(1),
            &rules,
            &cancel,
        );

        assert_eq!(removed, 0);
        assert_eq!(plan.included_files.len(), before);
        assert!(plan.excluded_files.is_empty());
    }

    #[test]
    fn an_empty_plan_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let options = ScanOptions::default();
        let (mut plan, rules) = plan_for(dir.path(), &options);
        assert!(plan.included_files.is_empty());

        let removed = apply_token_budget(
            &mut plan,
            dir.path(),
            &config_with_budget(10),
            &rules,
            &CancellationToken::new(),
        );

        assert_eq!(removed, 0);
    }

    #[test]
    fn every_file_is_accounted_for_after_the_pass() {
        // Nothing may be lost: each planned file ends up in exactly one of the two lists.
        let dir = fixture();
        let options = ScanOptions::default();
        let (mut plan, rules) = plan_for(dir.path(), &options);
        let before: Vec<String> = plan
            .included_files
            .iter()
            .map(|file| file.relative_path.clone())
            .collect();

        apply_token_budget(
            &mut plan,
            dir.path(),
            &config_with_budget(25),
            &rules,
            &CancellationToken::new(),
        );

        let mut after: Vec<String> = plan
            .included_files
            .iter()
            .chain(plan.excluded_files.iter())
            .map(|file| file.relative_path.clone())
            .collect();
        after.sort();
        let mut expected = before;
        expected.sort();
        assert_eq!(after, expected);
    }
}
