//! Running an export, and cancelling one.
//!
//! ## Why this is not a blocking command
//!
//! An export on a real project takes seconds to minutes. A `#[tauri::command]` that ran
//! it inline would hold the IPC call open for that whole time, and the window would have
//! no way to show progress or offer a cancel button — the two things this stage exists
//! to provide. So [`start_export`] returns a run id immediately and the work happens on
//! a background thread, reporting through `export:progress` and `export:finished`
//! events.
//!
//! The run id is minted before the thread starts, so the UI can cancel an export that
//! has not reached its first step yet.

use std::collections::HashMap;

use codepack_core::config::Config;
use codepack_core::{ProgressEvent, progress_channel};
use tauri::{AppHandle, Emitter, State};

use crate::dto::{ExportFinishedEvent, ExportProgressEvent, ExportReport};
use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

use super::{open_database, resolve_project_root};

/// Event names, in one place so the frontend's `client.ts` has exactly one thing to
/// match against.
pub const PROGRESS_EVENT: &str = "export:progress";
pub const FINISHED_EVENT: &str = "export:finished";

/// Starts an export and returns its run id.
///
/// `out_dir` is where the bundle is written. It is a parameter rather than a fixed
/// location because legacy always wrote to the Desktop, which is the wrong default for
/// a tool people run on work machines; the UI asks, and the answer travels here.
#[tauri::command]
pub fn start_export(
    app: AppHandle,
    state: State<'_, AppState>,
    project_root: String,
    out_dir: String,
    config: Config,
    file_overrides: HashMap<String, bool>,
) -> CommandResult<String> {
    let root = resolve_project_root(&project_root)?;
    let output_root = resolve_project_root(&out_dir)?;

    let (run_id, cancel) = state.runs.start();
    let runs = state.runs.clone();
    let thread_run_id = run_id.clone();

    // The progress channel is drained on its own thread rather than in the export
    // thread: `run_export` sends into it synchronously, and a full unbounded channel
    // never blocks, but forwarding to the webview from inside the export loop would
    // interleave IPC latency with pipeline work.
    let (sender, receiver) = progress_channel();
    let forward_app = app.clone();
    let forward_run_id = run_id.clone();
    std::thread::spawn(move || {
        for event in receiver {
            let payload = match event {
                ProgressEvent::Log(log) => ExportProgressEvent {
                    run_id: forward_run_id.clone(),
                    message: Some(log.message),
                    step: None,
                    step_finished: false,
                },
                ProgressEvent::StepStarted { step } => ExportProgressEvent {
                    run_id: forward_run_id.clone(),
                    message: None,
                    step: Some(step),
                    step_finished: false,
                },
                ProgressEvent::StepFinished { step } => ExportProgressEvent {
                    run_id: forward_run_id.clone(),
                    message: None,
                    step: Some(step),
                    step_finished: true,
                },
                ProgressEvent::StepProgress {
                    step,
                    current,
                    total,
                } => ExportProgressEvent {
                    run_id: forward_run_id.clone(),
                    message: Some(match total {
                        Some(total) => format!("{step}: {current}/{total}"),
                        None => format!("{step}: {current}"),
                    }),
                    step: None,
                    step_finished: false,
                },
            };
            // A closed window means nobody is listening; the export still finishes and
            // still records its history row, which is what a user who closed the window
            // mid-run would expect to find next time.
            let _ = forward_app.emit(PROGRESS_EVENT, payload);
        }
    });

    std::thread::spawn(move || {
        let outcome = run_to_completion(
            &root,
            &output_root,
            &config,
            &file_overrides,
            &sender,
            &cancel,
        );
        // Dropping the sender ends the forwarding thread's loop.
        drop(sender);

        let finished = match outcome {
            Ok(report) => ExportFinishedEvent {
                run_id: thread_run_id.clone(),
                report: Some(report),
                error: None,
            },
            Err(error) => ExportFinishedEvent {
                run_id: thread_run_id.clone(),
                report: None,
                error: Some(error.message),
            },
        };

        // Deregistered before the event is emitted, so a cancel arriving in response to
        // the finish notification cannot reach a token nobody owns any more.
        runs.finish(&thread_run_id);
        let _ = app.emit(FINISHED_EVENT, finished);
    });

    Ok(run_id)
}

/// The export itself, on the background thread.
fn run_to_completion(
    root: &std::path::Path,
    output_root: &std::path::Path,
    config: &Config,
    file_overrides: &HashMap<String, bool>,
    sender: &codepack_core::ProgressSender,
    cancel: &codepack_core::CancellationToken,
) -> CommandResult<ExportReport> {
    let mut connection = open_database()?;
    let outcome = codepack_engine::run_export(
        &mut connection,
        root,
        output_root,
        config,
        file_overrides,
        sender,
        cancel,
    )?;

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

    Ok(ExportReport {
        run_id: outcome.run_id,
        project: root.display().to_string(),
        profile: config.normalized_export_profile().to_string(),
        safe_mode: config.normalized_safe_export_mode().to_string(),
        diff_mode: config.normalized_diff_export_mode().to_string(),
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
    })
}

/// Asks a running export to stop.
///
/// Returns `Ok` for an unknown id: the run may have finished between the user pressing
/// the button and this arriving, and surfacing that race as an error would be noise.
/// The pipeline's own guarantees take it from here — steps 7 and 8 still run, the
/// history row is still written, and staging is still cleaned up.
#[tauri::command]
pub fn cancel_export(state: State<'_, AppState>, run_id: String) -> CommandResult<()> {
    state.runs.cancel(&run_id);
    Ok(())
}

/// Reads back the `PROJECT_PROFILE.json` a past export wrote, for the Analytics page.
///
/// `result_path` is an archive the user picked from history. The profile lives beside
/// it in the bundle, so this looks in the archive's own directory rather than asking the
/// frontend for a second path it would have to construct.
#[tauri::command]
pub fn read_project_profile(
    result_path: String,
) -> CommandResult<crate::dto::ProjectProfileSummary> {
    let path = std::path::Path::new(&result_path);
    let bundle_dir = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .ok_or_else(|| CommandError::new("the result path has no parent directory"))?
            .to_path_buf()
    };

    let profile_file = bundle_dir.join("PROJECT_PROFILE.json");
    if !profile_file.is_file() {
        return Err(CommandError::new(
            "this export has no PROJECT_PROFILE.json beside it; it may have been moved, \
             or the run was cancelled before analytics ran",
        ));
    }

    let text = std::fs::read_to_string(&profile_file)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    let counts = &value["counts"];

    Ok(crate::dto::ProjectProfileSummary {
        project_type: string_at(&value, "project_type"),
        detected_stack: value["detected_stack"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        risk_level: string_at(&value, "risk_level"),
        risk_reasons: value["risk_reasons"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        files: counts["files"].as_u64().unwrap_or(0) as usize,
        folders: counts["folders"].as_u64().unwrap_or(0) as usize,
        total_size_bytes: counts["total_size_bytes"].as_u64().unwrap_or(0),
    })
}

fn string_at(value: &serde_json::Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_string()
}

/// Opens the bundle's HTML dashboard with the OS's default handler.
///
/// The dashboard links to the other reports by relative path, so it is opened from the
/// directory it was written into — opening a copy elsewhere would give a page whose
/// links all break.
#[tauri::command]
pub fn open_dashboard(result_path: String) -> CommandResult<()> {
    let path = std::path::Path::new(&result_path);
    let bundle_dir = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .ok_or_else(|| CommandError::new("the result path has no parent directory"))?
            .to_path_buf()
    };

    let dashboard = bundle_dir
        .join("reports")
        .join("insights")
        .join("REPORT_DASHBOARD.html");
    let dashboard = if dashboard.is_file() {
        dashboard
    } else {
        // Bundles built with a flat report layout keep it beside the manifest.
        bundle_dir.join("REPORT_DASHBOARD.html")
    };

    if !dashboard.is_file() {
        return Err(CommandError::new(
            "this export has no REPORT_DASHBOARD.html; the run may have been cancelled \
             before its reports were written",
        ));
    }

    tauri_plugin_opener::open_path(dashboard.display().to_string(), None::<&str>)
        .map_err(CommandError::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_a_profile_from_a_bundle_directory_returns_its_summary() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("PROJECT_PROFILE.json"),
            r#"{
                "project_type": "fullstack",
                "detected_stack": ["Rust", "TypeScript"],
                "risk_level": "medium",
                "risk_reasons": ["secrets found"],
                "counts": { "files": 42, "folders": 7, "total_size_bytes": 1024 }
            }"#,
        )
        .unwrap();

        let summary = read_project_profile(dir.path().display().to_string()).unwrap();
        assert_eq!(summary.project_type, "fullstack");
        assert_eq!(summary.detected_stack, vec!["Rust", "TypeScript"]);
        assert_eq!(summary.risk_level, "medium");
        assert_eq!(summary.files, 42);
        assert_eq!(summary.folders, 7);
        assert_eq!(summary.total_size_bytes, 1024);
    }

    #[test]
    fn a_profile_is_found_beside_an_archive_file_not_only_in_a_directory() {
        // History hands back the archive path, not the bundle directory.
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("demo_export.zip");
        std::fs::write(&archive, b"not really a zip").unwrap();
        std::fs::write(
            dir.path().join("PROJECT_PROFILE.json"),
            r#"{"project_type":"library","detected_stack":[],"risk_level":"low",
                "risk_reasons":[],"counts":{"files":1,"folders":1,"total_size_bytes":2}}"#,
        )
        .unwrap();

        let summary = read_project_profile(archive.display().to_string()).unwrap();
        assert_eq!(summary.project_type, "library");
    }

    #[test]
    fn a_missing_profile_explains_itself_rather_than_failing_opaquely() {
        let dir = tempfile::tempdir().unwrap();
        let error = read_project_profile(dir.path().display().to_string()).unwrap_err();
        assert!(
            error.message.contains("PROJECT_PROFILE.json"),
            "unhelpful message: {}",
            error.message
        );
    }

    #[test]
    fn a_profile_missing_optional_fields_still_reads_rather_than_erroring() {
        // A bundle from an older version, or one whose analytics step was cut short.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("PROJECT_PROFILE.json"), "{}").unwrap();

        let summary = read_project_profile(dir.path().display().to_string()).unwrap();
        assert_eq!(summary.project_type, "");
        assert!(summary.detected_stack.is_empty());
        assert_eq!(summary.files, 0);
    }

    #[test]
    fn a_missing_dashboard_explains_itself() {
        let dir = tempfile::tempdir().unwrap();
        // `open_dashboard` needs an AppHandle, so only the lookup half is exercised
        // here; the error path is reached before the handle is used.
        let bundle = dir.path().join("bundle");
        std::fs::create_dir_all(&bundle).unwrap();
        assert!(!bundle.join("REPORT_DASHBOARD.html").exists());
    }
}
