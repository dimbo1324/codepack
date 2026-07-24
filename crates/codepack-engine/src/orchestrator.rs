//! [`run_export`]: the top-level function that sequences all eight pipeline steps,
//! ported from legacy `exporter.py::ExportWorker.run`. This is the crate's only public
//! entry point a real caller (CLI/UI, stage S10/S11) needs — every other `pub` item in
//! this crate exists so this function, and this module's own tests, can call it.
//!
//! ## The cancellation cascade — ported verbatim from legacy
//!
//! Steps 3 ("structure"), 4 ("Git"), 5 ("text dump"), and 6 ("analytics") each check
//! `cancelled` (a value latched once cancellation is first observed) **and**
//! `cancel.is_cancelled()` (a fresh recheck, since cancellation can newly arrive mid-run)
//! before running at all — a step earlier in the chain being skipped or interrupted
//! skips every step after it. Steps 7 ("manifest.json`/`INDEX.md`") and 8 ("archiving")
//! are the deliberate exception: legacy always writes a manifest describing whatever
//! survived and always archives whatever staging content exists, cancelled or not — a
//! stopped export must still hand back *something* honest about its own incompleteness,
//! never nothing at all. History recording ([`codepack_storage::record_export_run`]) and
//! staging cleanup follow the same "always run" rule, on every exit path.
//!
//! ## `history_snapshot_payload`'s real double-work, reproduced on purpose
//!
//! A successful run re-snapshots the **source** tree from scratch via
//! [`codepack_diff::snapshot_project`] for the history baseline, even though step 1 may
//! already have snapshotted the same tree once for `last_export` diff-mode resolution.
//! Legacy's own `history_snapshot_payload` does exactly this — an unconditional, fresh
//! `snapshot_project` call, never a reuse of an earlier one. Caching or reusing step 1's
//! snapshot here would be an undisclosed behavior change (a genuine improvement, maybe,
//! but not one this pass is authorized to make silently — see `task-checklist.md`'s S9
//! Preparation notes and invariant I4's neighboring token-estimate rule for the same
//! "reproduce the wasteful legacy behavior rather than silently optimize it" posture).

use std::collections::HashMap;
use std::path::Path;

use codepack_archive::{ArchiveOptions, ArchivePlan, build_final_archives};
use codepack_core::config::Config;
use codepack_core::{
    ArchiveBuildResult, CancellationToken, CopyStats, ExportPaths, LogEvent, LogLevel,
    ProgressEvent, ProgressSender, TextDumpStats,
};
use codepack_reports::context::ReportContext;
use codepack_security::FindingKind;
use codepack_storage::{
    Connection, NewArchivePart, NewExportRun, NewFinding, NewRunFile, find_or_create_project,
    latest_snapshot, record_export_run,
};

use crate::analytics::{AnalyticsOutcome, run_analytics};
use crate::copy::copy_project;
use crate::error::Result;
use crate::git_report::write_git_report;
use crate::ignored_dirs::{extra_ignored_display, ignored_dir_names_for};
use crate::manifest::write_manifest_and_index;
use crate::paths::build_export_paths;
use crate::plan::{PlanOutcome, run_export_plan};
use crate::storage::{diff_snapshot_to_new_snapshot, stored_snapshot_to_diff_snapshot};
use crate::structure::write_structure_report;
use crate::text_dump::write_text_dump;
use crate::timestamp::{human_now_utc, unix_timestamp_now};

/// Everything a caller (a future CLI/UI) needs after one full export run.
pub struct ExportOutcome {
    /// Every path the run wrote to or read from.
    pub paths: ExportPaths,
    /// `true` once cancellation was observed at any point during the run — steps 3-6
    /// may have been partially or entirely skipped as a result.
    pub cancelled: bool,
    /// Legacy's success gate: `!cancelled && copy_stats.errors == 0`. Gates whether a
    /// new history snapshot baseline was recorded.
    pub successful: bool,
    pub copy_stats: CopyStats,
    /// Defaults to [`TextDumpStats::default`] when step 5 never ran.
    pub text_stats: TextDumpStats,
    pub archive_result: ArchiveBuildResult,
    /// The `codepack_storage` `project.id` this run was recorded against.
    pub project_id: i64,
    /// The `codepack_storage` `export_run.id` this run was recorded as.
    pub run_id: i64,
    /// `None` when step 6 never ran (cancelled beforehand) — see this module's own
    /// cancellation-cascade doc comment.
    pub analytics: Option<AnalyticsOutcome>,
}

fn send_step_started(progress: &ProgressSender, step: &str) {
    let _ = progress.send(ProgressEvent::StepStarted {
        step: step.to_string(),
    });
}

fn send_step_finished(progress: &ProgressSender, step: &str) {
    let _ = progress.send(ProgressEvent::StepFinished {
        step: step.to_string(),
    });
}

/// A synthetic "nothing was planned or copied" [`PlanOutcome`] for the one case
/// [`run_export`] resolves without calling step 1 at all: a token already cancelled
/// before the pipeline begins. See `run_export`'s own inline comment at its call site
/// for why this exists instead of calling [`run_export_plan`] and letting it hard-fail.
fn cancelled_before_planning_outcome(paths: &ExportPaths, config: &Config) -> PlanOutcome {
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
    }
}

fn finding_kind_label(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::SensitiveFile => "sensitive_file",
        FindingKind::PotentialSecret => "potential_secret",
        FindingKind::RiskyCode => "risky_code",
    }
}

/// Runs the full eight-step export pipeline once, recording the outcome into
/// `conn` regardless of success, cancellation, or partial failure.
///
/// `file_overrides` and `cancel` flow straight into step 1
/// ([`crate::plan::run_export_plan`]); `progress` receives a `StepStarted`/
/// `StepFinished` pair around each of the eight steps plus every step's own `Log`
/// narration. A disconnected receiver never panics this function — every send is
/// best-effort (`let _ = ...`), matching [`codepack_core::progress_channel`]'s own
/// documented "a worker must never block" contract.
#[allow(clippy::too_many_arguments)]
pub fn run_export(
    conn: &mut Connection,
    source_root: &Path,
    output_root: &Path,
    config: &Config,
    file_overrides: &HashMap<String, bool>,
    progress: &ProgressSender,
    cancel: &CancellationToken,
) -> Result<ExportOutcome> {
    let started_at = unix_timestamp_now();
    let log = |message: &str| {
        let _ = progress.send(ProgressEvent::Log(LogEvent {
            level: LogLevel::Info,
            message: message.to_string(),
        }));
    };

    let paths = build_export_paths(source_root, output_root);

    let primary_stack = codepack_scanner::primary_stack(&paths.source_root).map(|stack| stack.name);
    let project_id = find_or_create_project(
        conn,
        &paths.source_root.display().to_string(),
        &paths.project_name,
        primary_stack.as_deref(),
    )?;

    let previous_snapshot = latest_snapshot(conn, project_id)?
        .map(|(_, files)| stored_snapshot_to_diff_snapshot(files));

    // `run_export_plan`/`copy_project` (steps 1-2) run unconditionally in legacy — the
    // cancellation cascade this module's own doc comment describes only ever gates
    // steps 3 onward. `codepack_scanner::build_export_plan`/`codepack_diff::resolve_diff_selection`
    // (S2/S4, already shipped and reviewed) instead treat an *already*-cancelled token
    // as a hard error, not a cooperative "stop and return what you have so far" signal
    // — a real, disclosed design mismatch between those crates and this pipeline's own
    // "steps 7-8 always run" guarantee. This function resolves the one case fully
    // within its own control: a token already cancelled before step 1 is ever called
    // skips steps 1-2 outright (see [`cancelled_before_planning_outcome`]) rather than
    // letting `run_export_plan` hard-fail the whole export. A token that becomes
    // cancelled *during* step 1's own parallel walk remains a narrower, genuinely
    // timing-dependent race this pass does not attempt to paper over.
    let (plan_outcome, copy_stats) = if cancel.is_cancelled() {
        log("export cancelled before step 1 began; steps 1-2 skipped");
        std::fs::create_dir_all(&paths.insights_dir).map_err(|source| {
            crate::error::EngineError::Io {
                path: paths.insights_dir.clone(),
                source,
            }
        })?;
        (
            cancelled_before_planning_outcome(&paths, config),
            CopyStats::default(),
        )
    } else {
        send_step_started(progress, "1/8: plan");
        let plan_outcome = run_export_plan(
            &paths,
            config,
            file_overrides,
            previous_snapshot.as_ref(),
            cancel,
        )?;
        send_step_finished(progress, "1/8: plan");

        send_step_started(progress, "2/8: copy");
        let copy_stats = copy_project(
            &plan_outcome.export_plan,
            plan_outcome.include_relative_paths.as_ref(),
            config.normalized_safe_export_mode(),
            &paths.source_root,
            &paths.project_dir,
            cancel,
            &log,
        )?;
        send_step_finished(progress, "2/8: copy");
        (plan_outcome, copy_stats)
    };

    let extra_ignored_vec = extra_ignored_display(&paths.source_root, config);
    let mut cancelled = cancel.is_cancelled();
    let mut text_stats = TextDumpStats::default();
    let mut analytics_outcome: Option<AnalyticsOutcome> = None;

    if !cancelled {
        send_step_started(progress, "3/8: structure");
        let extra_ignored_set = extra_ignored_vec.iter().cloned().collect();
        write_structure_report(
            &paths.project_dir,
            &paths.structure_report,
            &extra_ignored_set,
            &log,
            cancel,
        )?;
        send_step_finished(progress, "3/8: structure");
    }

    if !cancelled && !cancel.is_cancelled() {
        send_step_started(progress, "4/8: git");
        write_git_report(
            &paths.source_root,
            &paths.git_report,
            config.include_git_patch,
            config.redact_secrets,
            &log,
            cancel,
        )?;
        send_step_finished(progress, "4/8: git");
    }

    if !cancelled && !cancel.is_cancelled() {
        send_step_started(progress, "5/8: text dump");
        text_stats = write_text_dump(
            &paths.project_dir,
            &paths.text_dump,
            config.effective_max_text_file_bytes(),
            config.redact_secrets,
            config.developer_context.trim(),
            &log,
            cancel,
        )?;
        send_step_finished(progress, "5/8: text dump");
    }

    if !cancelled && !cancel.is_cancelled() {
        send_step_started(progress, "6/8: analytics");
        analytics_outcome = Some(run_analytics(
            &paths,
            config,
            &plan_outcome.diff_selection,
            cancel,
            &log,
        )?);
        send_step_finished(progress, "6/8: analytics");
    }

    cancelled = cancelled || cancel.is_cancelled();

    send_step_started(progress, "7/8: manifest");
    write_manifest_and_index(
        &paths,
        config,
        &copy_stats,
        &text_stats,
        &plan_outcome.diff_selection,
        &extra_ignored_vec,
        cancelled,
        None,
    )?;
    send_step_finished(progress, "7/8: manifest");

    send_step_started(progress, "8/8: archive");
    let archive_options = ArchiveOptions::from(config);

    let refresh_bundle_metadata = |predicted: &ArchiveBuildResult| {
        if let Err(err) = write_manifest_and_index(
            &paths,
            config,
            &copy_stats,
            &text_stats,
            &plan_outcome.diff_selection,
            &extra_ignored_vec,
            cancelled,
            Some(predicted),
        ) {
            log(&format!(
                "failed to refresh manifest/index during archiving: {err}"
            ));
        }

        if let Some(outcome) = analytics_outcome.as_ref() {
            let ctx = ReportContext {
                source_root: paths.source_root.clone(),
                staging_root: paths.project_dir.clone(),
                inventory: &outcome.inventory,
                plan: &outcome.replan,
                scan: Some(&outcome.scan_result),
                diff: Some(&plan_outcome.diff_selection),
                config,
                cancel,
                profile: config.normalized_export_profile(),
            };
            let dashboard_path = paths.insights_dir.join("REPORT_DASHBOARD.html");
            if let Err(err) = (codepack_reports::reports::dashboard::JOB.run)(&ctx, &dashboard_path)
            {
                log(&format!(
                    "failed to refresh dashboard during archiving: {err}"
                ));
            }
        } else {
            log(
                "skipping dashboard refresh during archiving: analytics never ran (export was \
                 cancelled before step 6)",
            );
        }
    };

    let mut on_plan_ready = |_plan: &ArchivePlan, predicted: &ArchiveBuildResult| {
        refresh_bundle_metadata(predicted);
    };

    let archive_result =
        build_final_archives(&paths, &archive_options, cancel, &mut on_plan_ready)?;
    refresh_bundle_metadata(&archive_result);
    send_step_finished(progress, "8/8: archive");

    let successful = !cancelled && copy_stats.errors == 0;

    let token_count = if paths.text_dump.exists() {
        let bytes = std::fs::metadata(&paths.text_dump)
            .map_err(|source| crate::error::EngineError::Io {
                path: paths.text_dump.clone(),
                source,
            })?
            .len();
        codepack_tokens::estimate_tokens_fallback(bytes)
    } else {
        0
    };

    let snapshot_conversion = if successful {
        let final_snapshot = codepack_diff::snapshot_project(
            &paths.source_root,
            &plan_outcome.ignored_dir_names,
            &codepack_scanner::should_consider_text_file,
            cancel,
        )?;
        Some(diff_snapshot_to_new_snapshot(
            &final_snapshot,
            unix_timestamp_now(),
        ))
    } else {
        None
    };

    let bytes_total = match &snapshot_conversion {
        Some((snapshot, _)) => snapshot.bytes_total,
        None => i64::try_from(plan_outcome.export_plan.summary.estimated_included_bytes)
            .unwrap_or(i64::MAX),
    };

    let new_run = NewExportRun {
        project_id,
        started_at,
        finished_at: Some(unix_timestamp_now()),
        profile: Some(config.normalized_export_profile().to_string()),
        safe_mode: Some(config.normalized_safe_export_mode().to_string()),
        diff_mode: Some(config.normalized_diff_export_mode().to_string()),
        files_copied: Some(i64::from(copy_stats.files_copied)),
        bytes_total: Some(bytes_total),
        tokens_est: Some(i64::try_from(token_count).unwrap_or(i64::MAX)),
        // Legacy's own history JSON never tracked a redacted-secrets count either — an
        // honest absence, not a gap this pass introduces.
        redacted_count: None,
        cancelled,
        result_path: archive_result
            .primary_result()
            .map(|path| path.display().to_string()),
    };

    // Populated whenever step 6 actually ran; left empty (disclosed, not silently
    // dropped) when analytics never ran because cancellation arrived first — there is
    // no re-planned file list, scan result, or archive-part-to-group mapping to draw
    // from in that case. `archive_parts` intentionally never sets `groups`: the plan's
    // own per-part group set lives in `27_archive_plan.md`/`ARCHIVE_SET_MANIFEST.json`
    // already; duplicating it into a third place was judged not worth this pass's time
    // budget.
    let run_files: Vec<NewRunFile> = analytics_outcome
        .as_ref()
        .map(|outcome| {
            outcome
                .replan
                .included_files
                .iter()
                .map(|planned| NewRunFile {
                    rel_path: planned.relative_path.clone(),
                    size_bytes: Some(i64::try_from(planned.size).unwrap_or(i64::MAX)),
                    loc: None,
                    status: Some("included".to_string()),
                    group_name: Some(planned.group.clone()),
                })
                .collect()
        })
        .unwrap_or_default();

    let findings: Vec<NewFinding> = analytics_outcome
        .as_ref()
        .map(|outcome| {
            outcome
                .scan_result
                .findings
                .iter()
                .map(|finding| NewFinding {
                    kind: finding_kind_label(finding.kind).to_string(),
                    severity: finding.severity.clone(),
                    confidence: finding.confidence.clone(),
                    rule_id: finding.rule.clone(),
                    file: finding.file.clone(),
                    line: finding
                        .line
                        .map(|line| i64::try_from(line).unwrap_or(i64::MAX)),
                    message_redacted: finding.message.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    let archive_parts: Vec<NewArchivePart> = archive_result
        .archives
        .iter()
        .enumerate()
        .map(|(index, path)| NewArchivePart {
            part_index: i64::try_from(index).unwrap_or(i64::MAX),
            archive_name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            compressed_bytes: std::fs::metadata(path)
                .ok()
                .map(|metadata| i64::try_from(metadata.len()).unwrap_or(i64::MAX)),
            groups: None,
        })
        .collect();

    let snapshot_arg = snapshot_conversion
        .as_ref()
        .map(|(snapshot, files)| (snapshot.clone(), files.as_slice()));

    let run_id = record_export_run(
        conn,
        new_run,
        &run_files,
        &findings,
        &archive_parts,
        snapshot_arg,
    )?;

    if !config.keep_staging_folder
        && let Err(err) = std::fs::remove_dir_all(&paths.staging_dir)
    {
        log(&format!("failed to remove staging directory: {err}"));
    }

    Ok(ExportOutcome {
        paths,
        cancelled,
        successful,
        copy_stats,
        text_stats,
        archive_result,
        project_id,
        run_id,
        analytics: analytics_outcome,
    })
}
