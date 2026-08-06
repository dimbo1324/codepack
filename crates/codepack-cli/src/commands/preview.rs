//! `codepack preview` — what an export *would* do.
//!
//! Writes nothing, anywhere. That is the whole point: a user deciding whether it is
//! safe to share this project must be able to ask without producing a bundle, and
//! without a report file appearing in their repository.

use std::collections::HashMap;

use codepack_core::CancellationToken;
use serde::Serialize;

use crate::cli::PreviewArgs;
use crate::commands::{self, ProjectContext};
use crate::error::Result;
use crate::exit::Outcome;
use crate::output::{self, Format};
use crate::settings::ResolutionTrace;

#[derive(Debug, Serialize)]
pub(crate) struct PreviewReport {
    pub project: String,
    pub profile: String,
    pub safe_mode: String,
    pub diff_mode: String,
    pub included_files: usize,
    pub excluded_files: usize,
    pub skipped_dirs: usize,
    pub estimated_bytes: u64,
    pub estimated_bytes_human: String,
    pub estimated_tokens: u64,
    /// Files excluded because they look sensitive — the reason a preview exists.
    pub sensitive_exclusions: Vec<SensitiveExclusion>,
    pub dropped_by_budget: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    pub resolution: ResolutionTrace,
}

#[derive(Debug, Serialize)]
pub(crate) struct SensitiveExclusion {
    pub path: String,
    pub severity: String,
    pub reason: String,
}

pub(crate) fn run(args: &PreviewArgs, format: Format) -> Result<Outcome> {
    let context = commands::prepare(&args.project)?;
    let report = build(&context, args.list_files)?;

    if format.is_json() {
        output::emit_json("preview", &report)?;
    } else {
        print_human(&report);
    }
    Ok(Outcome::Success)
}

pub(crate) fn build(context: &ProjectContext, list_files: bool) -> Result<PreviewReport> {
    // No previous snapshot is consulted: `preview` must not touch the history
    // database, so a `last_export` diff degrades to "everything", which
    // `codepack-diff` already reports as a warning in its own selection.
    let outcome = codepack_engine::plan_export(
        &context.root,
        &context.config,
        &HashMap::new(),
        None,
        &CancellationToken::new(),
    )?;

    let plan = &outcome.export_plan;
    let sensitive_exclusions = plan
        .sensitive_warnings()
        .into_iter()
        .map(|file| SensitiveExclusion {
            path: file.relative_path.clone(),
            severity: file.severity.clone(),
            reason: file.reason.clone(),
        })
        .collect();

    let estimated_bytes = plan.summary.estimated_included_bytes;
    Ok(PreviewReport {
        project: context.root.display().to_string(),
        profile: context.config.normalized_export_profile().to_string(),
        safe_mode: context.config.normalized_safe_export_mode().to_string(),
        diff_mode: outcome.diff_selection.mode.clone(),
        included_files: plan.included_files.len(),
        excluded_files: plan.excluded_files.len(),
        skipped_dirs: plan.skipped_dirs.len(),
        estimated_bytes,
        estimated_bytes_human: codepack_tokens::format_bytes(estimated_bytes),
        estimated_tokens: codepack_tokens::estimate_tokens_fallback(estimated_bytes),
        sensitive_exclusions,
        dropped_by_budget: outcome.dropped_by_budget,
        files: list_files.then(|| {
            plan.included_files
                .iter()
                .map(|file| file.relative_path.clone())
                .collect()
        }),
        resolution: context.resolution_for_output(),
    })
}

fn print_human(report: &PreviewReport) {
    output::line(format!("Project:   {}", report.project));
    output::line(format!(
        "Settings:  profile={} safe-mode={} diff={}",
        report.profile, report.safe_mode, report.diff_mode
    ));
    output::line("");
    output::line(format!(
        "Would include {} file(s), {} ({} tokens est.)",
        report.included_files, report.estimated_bytes_human, report.estimated_tokens
    ));
    output::line(format!(
        "Excluded {} file(s), skipped {} director(ies)",
        report.excluded_files, report.skipped_dirs
    ));
    if report.dropped_by_budget > 0 {
        output::line(format!(
            "Token budget dropped {} file(s)",
            report.dropped_by_budget
        ));
    }

    if report.sensitive_exclusions.is_empty() {
        output::line("");
        output::line("No sensitive files were excluded.");
    } else {
        output::line("");
        output::line(format!(
            "Sensitive files kept out of the export ({}):",
            report.sensitive_exclusions.len()
        ));
        for item in &report.sensitive_exclusions {
            output::line(format!(
                "  [{}] {} — {}",
                item.severity, item.path, item.reason
            ));
        }
    }

    if let Some(files) = &report.files {
        output::line("");
        output::line("Included files:");
        for path in files {
            output::line(format!("  {path}"));
        }
    }
}
