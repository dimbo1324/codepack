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
//! The whole pass is skipped unless `Config::token_budget` is non-zero, which is the
//! default, so an export that does not ask for a budget pays nothing for this module.

use std::collections::BTreeMap;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_reports::context::{Inventory, ReportContext};
use codepack_scanner::ExportPlan;
use codepack_tokens::{BudgetCandidate, estimate_tokens_fallback};

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
    cancel: &CancellationToken,
) -> usize {
    if config.token_budget == 0 || plan.included_files.is_empty() {
        return 0;
    }

    let ranking = importance_of_planned_files(plan, source_root, config, cancel);

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
    let mut included = Vec::with_capacity(kept.len());
    let mut removed = 0usize;
    for file in std::mem::take(&mut plan.included_files) {
        if kept.contains(file.relative_path.as_str()) {
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
