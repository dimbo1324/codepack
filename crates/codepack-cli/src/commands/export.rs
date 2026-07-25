//! `codepack export` — the full pipeline.
//!
//! Thin by design: every decision lives in `codepack-engine`, and this module's job is
//! to hand it a resolved configuration, stream its progress to stderr, and turn its
//! outcome into a report and an exit code.

use std::collections::HashMap;
use std::path::PathBuf;

use codepack_core::{CancellationToken, ProgressEvent};
use serde::Serialize;

use crate::cli::ExportArgs;
use crate::commands;
use crate::error::{CliError, Result};
use crate::exit::Outcome;
use crate::output::{self, Format};
use crate::settings::ResolutionTrace;

#[derive(Debug, Serialize)]
pub(crate) struct ExportReport {
    pub project: String,
    pub profile: String,
    pub safe_mode: String,
    pub diff_mode: String,
    /// `false` when the run was cancelled or hit copy errors. The bundle may still
    /// exist — steps 7 and 8 always run — but it describes an incomplete export.
    pub successful: bool,
    pub cancelled: bool,
    pub files_copied: u32,
    pub files_skipped: u32,
    pub errors: u32,
    /// The archive a user should pick up, or `null` if none was produced.
    pub result_path: Option<String>,
    pub archives: Vec<String>,
    /// Where the bundle was assembled, useful when no single archive was produced.
    pub staging_dir: String,
    pub critical_findings: usize,
    pub run_id: i64,
    pub resolution: ResolutionTrace,
}

pub(crate) fn run(args: &ExportArgs, format: Format) -> Result<Outcome> {
    let context = commands::prepare(&args.project)?;
    let output_root = resolve_output_root(args.out.as_deref())?;

    let mut conn = commands::open_history_db()?;
    let (progress, events) = codepack_core::progress_channel();

    // Progress goes to stderr so `--json` stdout stays a single parseable document.
    // The receiver runs on its own thread because the export fills the channel as it
    // goes; draining afterwards would buffer the whole run in memory and show the user
    // nothing until it finished.
    let quiet = format.is_json();
    let printer = std::thread::spawn(move || {
        for event in events {
            if let ProgressEvent::Log(log) = event {
                if !quiet {
                    output::note(format!("  {}", log.message));
                }
            } else if let ProgressEvent::StepStarted { step } = event {
                output::note(format!("→ {step}"));
            }
        }
    });

    let outcome = codepack_engine::run_export(
        &mut conn,
        &context.root,
        &output_root,
        &context.config,
        &HashMap::new(),
        &progress,
        &CancellationToken::new(),
    );

    // Dropping the sender ends the printer's loop; do it before unwrapping the result
    // so a failed export still joins the thread instead of leaking it.
    drop(progress);
    let _ = printer.join();

    let outcome = outcome?;
    let critical_findings = outcome
        .analytics
        .as_ref()
        .map(|analytics| {
            analytics
                .scan_result
                .findings
                .iter()
                .filter(|finding| finding.severity == "critical")
                .count()
        })
        .unwrap_or(0);

    let report = ExportReport {
        project: context.root.display().to_string(),
        profile: context.config.normalized_export_profile().to_string(),
        safe_mode: context.config.normalized_safe_export_mode().to_string(),
        diff_mode: context.config.normalized_diff_export_mode().to_string(),
        successful: outcome.successful,
        cancelled: outcome.cancelled,
        files_copied: outcome.copy_stats.files_copied,
        files_skipped: outcome.copy_stats.files_skipped,
        errors: outcome.copy_stats.errors,
        result_path: outcome
            .archive_result
            .primary_result()
            .map(|path| path.display().to_string()),
        archives: outcome
            .archive_result
            .archives
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        staging_dir: outcome.paths.staging_dir.display().to_string(),
        critical_findings,
        run_id: outcome.run_id,
        resolution: context.resolution_for_output(),
    };

    if format.is_json() {
        output::emit_json("export", &report)?;
    } else {
        print_human(&report);
    }

    // An export that produced a bundle containing critical findings still succeeded as
    // an export; the exit code reports what was found, which is what a pipeline gates
    // on. A failed export is a plain failure and never reaches here.
    Ok(if report.critical_findings > 0 {
        Outcome::CriticalSecretsFound
    } else {
        Outcome::Success
    })
}

/// `--out` defaults to the current directory rather than to the desktop: legacy was a
/// Windows desktop application, this is a tool run from a shell, very often in CI where
/// there is no desktop at all.
fn resolve_output_root(requested: Option<&std::path::Path>) -> Result<PathBuf> {
    let path = match requested {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir().map_err(|source| CliError::Read {
            path: PathBuf::from("."),
            source,
        })?,
    };
    std::fs::create_dir_all(&path).map_err(|source| CliError::Read {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

fn print_human(report: &ExportReport) {
    output::line("");
    if report.successful {
        output::line("Export complete.");
    } else if report.cancelled {
        output::line("Export cancelled — the bundle describes an incomplete run.");
    } else {
        output::line("Export finished with errors — the bundle is incomplete.");
    }

    output::line(format!(
        "  {} file(s) copied, {} skipped, {} error(s)",
        report.files_copied, report.files_skipped, report.errors
    ));
    match &report.result_path {
        Some(path) => output::line(format!("  Result: {path}")),
        None => output::line(format!("  Bundle: {}", report.staging_dir)),
    }
    if report.archives.len() > 1 {
        output::line(format!("  Split into {} parts", report.archives.len()));
    }
    if report.critical_findings > 0 {
        output::line(format!(
            "  {} critical finding(s) — see reports/insights/06_security_scan.txt",
            report.critical_findings
        ));
    }
}
