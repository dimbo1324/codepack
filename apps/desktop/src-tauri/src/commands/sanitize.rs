//! Running a "Sterile copy", and cancelling one.
//!
//! Mirrors `commands::export`'s background-thread pattern, for the same reason: turning
//! a whole project into a stripped-and-formatted copy can take from seconds to minutes,
//! and a blocking `#[tauri::command]` would freeze the window for that whole time.
//! [`start_sanitize`] therefore mints a run id, registers its cancellation token, spawns
//! the work on a background thread, and returns immediately.
//!
//! Unlike an export, there is no `sanitize:progress` stream. `codepack_sanitize::
//! run_sterile_copy` processes every file in one `rayon` sweep and reports no
//! intermediate progress today (`crates/codepack-sanitize/src/run.rs`), so a progress
//! channel here would be a UI promise the backend cannot keep. `sanitize:finished` is
//! the only event, carrying either the full report or the (already redacted) error.
//! Cancellation still works at any moment despite the missing progress feed: the run is
//! registered in [`AppState::runs`] before the thread starts, and `codepack-sanitize`
//! checks the same cancellation token both before building its file plan and inside its
//! per-file loop.

use codepack_sanitize::{FileOutcome, SterileCopyOptions, SterileCopyReport, run_sterile_copy};
use tauri::{AppHandle, Emitter, State};

use crate::dto::{
    SanitizeArchive, SanitizeFileOutcome, SanitizeFinishedEvent, SanitizeReport, SanitizeSummary,
};
use crate::error::{CommandError, CommandResult};
use crate::state::AppState;

use super::resolve_project_root;

/// Event name, matching `sanitize:finished` in the frontend's `client.ts`.
pub const FINISHED_EVENT: &str = "sanitize:finished";

/// Starts a sterile copy and returns its run id.
///
/// `destination_root` is not validated with [`resolve_project_root`] the way
/// `source_root` is: unlike a project root, the destination is allowed not to exist yet
/// — `run_sterile_copy` creates it. The source/destination-overlap guard (invariant
/// I2-style) still runs, just inside the background thread rather than here, and its
/// rejection reaches the frontend as a `sanitize:finished` error rather than as this
/// call's own `Err`, exactly like an export's own destination problems do.
#[tauri::command]
pub fn start_sanitize(
    app: AppHandle,
    state: State<'_, AppState>,
    source_root: String,
    destination_root: String,
    safety_mode: String,
    archive_path: Option<String>,
) -> CommandResult<String> {
    let source = resolve_project_root(&source_root)?;

    let destination = std::path::PathBuf::from(&destination_root);
    if destination.as_os_str().is_empty() {
        return Err(CommandError::new("no destination folder was given"));
    }

    // An empty string is what an untouched text field sends; treating it as "no
    // archive" rather than as a path avoids a run failing on a path of `""`.
    let archive = archive_path
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from);

    let (run_id, cancel) = state.runs.start();
    let thread_run_id = run_id.clone();
    let runs = state.runs.clone();

    std::thread::spawn(move || {
        let options = SterileCopyOptions {
            source_root: source,
            destination_root: destination,
            safety_mode,
            archive_path: archive,
            cancellation: cancel,
        };

        let finished = match run_sterile_copy(&options) {
            Ok(report) => SanitizeFinishedEvent {
                run_id: thread_run_id.clone(),
                report: Some(assemble(&options, &report)),
                error: None,
            },
            Err(error) => SanitizeFinishedEvent {
                run_id: thread_run_id.clone(),
                report: None,
                error: Some(CommandError::new(error).message),
            },
        };

        // Deregistered before the event is emitted, for the same race-avoidance reason
        // `export::start_export` documents: a cancel arriving in response to the finish
        // notification must not reach a token nobody owns any more.
        runs.finish(&thread_run_id);
        let _ = app.emit(FINISHED_EVENT, finished);
    });

    Ok(run_id)
}

/// Asks a running sterile copy to stop. An unknown id is not an error — see
/// `export::cancel_export` for the identical race.
#[tauri::command]
pub fn cancel_sanitize(state: State<'_, AppState>, run_id: String) -> CommandResult<()> {
    state.runs.cancel(&run_id);
    Ok(())
}

fn assemble(options: &SterileCopyOptions, result: &SterileCopyReport) -> SanitizeReport {
    SanitizeReport {
        source: options.source_root.display().to_string(),
        destination: options.destination_root.display().to_string(),
        safety_mode: options.safety_mode.clone(),
        archive: result.archive.as_ref().map(|archive| SanitizeArchive {
            path: archive.path.display().to_string(),
            file_count: archive.file_count,
            bytes: archive.bytes,
        }),
        summary: SanitizeSummary {
            total_files: result.summary.total_files,
            stripped_and_formatted: result.summary.stripped_and_formatted,
            stripped_only_no_formatter_found: result.summary.stripped_only_no_formatter_found,
            skipped_unsupported_language: result.summary.skipped_unsupported_language,
            skipped_sensitive_or_redacted: result.summary.skipped_sensitive_or_redacted,
            errors: result.summary.errors,
        },
        files: result
            .per_file
            .iter()
            .map(|(path, outcome)| SanitizeFileOutcome {
                path: path.to_string_lossy().replace('\\', "/"),
                outcome: outcome.label(),
                detail: detail_of(outcome),
            })
            .collect(),
    }
}

/// Same shape the CLI's `codepack sanitize` renders per file
/// (`crates/codepack-cli/src/commands/sanitize.rs`), reused rather than reinvented so
/// the two front ends describe the same outcome the same way.
fn detail_of(outcome: &FileOutcome) -> Option<String> {
    match outcome {
        FileOutcome::StrippedAndFormatted {
            language,
            formatter,
        } => Some(format!("{language} → {formatter}")),
        FileOutcome::StrippedOnlyNoFormatterFound { language } => Some(language.clone()),
        FileOutcome::SkippedUnsupportedLanguage { reason } => Some(reason.clone()),
        FileOutcome::SkippedSensitiveOrRedacted => None,
        FileOutcome::Error { message } => Some(message.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(source: std::path::PathBuf, destination: std::path::PathBuf) -> SterileCopyOptions {
        SterileCopyOptions {
            source_root: source,
            destination_root: destination,
            safety_mode: "safe".to_string(),
            archive_path: None,
            cancellation: codepack_core::CancellationToken::new(),
        }
    }

    #[test]
    fn an_archive_reaches_the_frontend_with_its_byte_size_unformatted() {
        // The frontend formats bytes itself (`util/format.ts` mirrors
        // `codepack_tokens::format_bytes`), so this layer must hand over the number —
        // invariant I4: byte-based reporting is preserved, never replaced.
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let opts = options(
            source.path().to_path_buf(),
            destination.path().to_path_buf(),
        );

        let result = SterileCopyReport {
            per_file: Vec::new(),
            summary: codepack_sanitize::SterileCopySummary::default(),
            archive: Some(codepack_sanitize::SterileCopyArchive {
                path: std::path::PathBuf::from("C:\\out\\sterile.7z"),
                file_count: 12,
                bytes: 4096,
            }),
        };

        let archive = assemble(&opts, &result)
            .archive
            .expect("the archive must survive the DTO conversion");
        assert_eq!(archive.file_count, 12);
        assert_eq!(archive.bytes, 4096);
        assert!(archive.path.ends_with("sterile.7z"));
    }

    #[test]
    fn assemble_carries_the_summary_and_every_file_outcome() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let opts = options(
            source.path().to_path_buf(),
            destination.path().to_path_buf(),
        );

        let per_file = vec![
            (
                std::path::PathBuf::from("main.rs"),
                FileOutcome::StrippedAndFormatted {
                    language: "Rust".to_string(),
                    formatter: "rustfmt".to_string(),
                },
            ),
            (
                std::path::PathBuf::from(".env"),
                FileOutcome::SkippedSensitiveOrRedacted,
            ),
        ];
        let summary = codepack_sanitize::SterileCopySummary::from_outcomes(
            per_file.iter().map(|(_, outcome)| outcome),
        );
        let result = SterileCopyReport {
            per_file,
            summary,
            archive: None,
        };

        let report = assemble(&opts, &result);
        assert_eq!(report.summary.total_files, 2);
        assert_eq!(report.summary.stripped_and_formatted, 1);
        assert_eq!(report.summary.skipped_sensitive_or_redacted, 1);
        assert_eq!(report.files.len(), 2);
        assert_eq!(report.files[0].path, "main.rs");
        assert_eq!(report.files[0].outcome, "stripped_and_formatted");
        assert_eq!(report.files[0].detail.as_deref(), Some("Rust → rustfmt"));
        assert_eq!(report.files[1].outcome, "skipped_sensitive_or_redacted");
        assert!(report.files[1].detail.is_none());
    }
}
