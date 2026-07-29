//! `codepack sanitize` — the "Sterile copy" standalone action.
//!
//! Not a flavor of `export`: it writes a plain folder, not a bundle, and never touches
//! the archive/report artifacts `export` produces (`docs/decisions/open-questions.md`,
//! 2026-07-28). All of the actual work — file selection, safety filtering, redaction,
//! tree-sitter comment stripping, `PATH`-formatter reformatting — lives in
//! `codepack-sanitize`; this module only parses arguments and renders its result.

use codepack_core::CancellationToken;
use codepack_sanitize::{FileOutcome, SterileCopyOptions, SterileCopyReport, run_sterile_copy};
use serde::Serialize;

use crate::cli::SanitizeArgs;
use crate::error::{CliError, Result};
use crate::exit::Outcome;
use crate::output::{self, Format};

#[derive(Debug, Serialize)]
pub(crate) struct SanitizeReport {
    pub source: String,
    /// `None` when `--out` was omitted: the copy went to a scratch folder that has
    /// since been removed, and naming a path that no longer exists would be worse than
    /// saying nothing. The archive is the result in that case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    pub safety_mode: String,
    /// Present only when `--archive` asked for one. Reported next to the destination
    /// because when `--out` was omitted the destination was a temporary folder that no
    /// longer exists, and the archive is the only result the user can open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<ArchiveInfo>,
    pub summary: Summary,
    pub files: Vec<ReportedFile>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ArchiveInfo {
    pub path: String,
    /// The container actually written — the file name may have chosen it.
    pub format: &'static str,
    pub file_count: usize,
    pub bytes: u64,
    pub bytes_human: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct Summary {
    pub total_files: usize,
    pub stripped_and_formatted: usize,
    pub stripped_only_no_formatter_found: usize,
    pub skipped_unsupported_language: usize,
    pub skipped_sensitive_or_redacted: usize,
    pub errors: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReportedFile {
    pub path: String,
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

pub(crate) fn run(args: &SanitizeArgs, format: Format) -> Result<Outcome> {
    let safety_mode = args
        .safe_mode
        .map(|mode| mode.as_config_value().to_string())
        .unwrap_or_else(|| "safe".to_string());

    // When `--out` was omitted the copy goes to a scratch folder that exists only long
    // enough to be packed. `_scratch` is bound for the whole call rather than dropped
    // at the end of this statement: dropping a `TempDir` deletes it, and the archive is
    // written from that folder further down.
    //
    // Written as an `Option` the destination is *derived* from, rather than as a match
    // over both, so the "neither" case that clap already rejects cannot be reached in
    // code at all — no `unreachable!` to be wrong about later.
    let (_scratch, destination) = match &args.out {
        Some(out) => (None, out.clone()),
        None => {
            let dir = scratch_beside_archive(args)?;
            let path = dir.path().to_path_buf();
            (Some(dir), path)
        }
    };

    let options = SterileCopyOptions {
        source_root: args.source.clone(),
        destination_root: destination.clone(),
        safety_mode: safety_mode.clone(),
        archive_path: args.archive.clone(),
        archive_format: args.archive_format.map(|format| {
            codepack_sanitize::ArchiveFormat::from_config_value(format.as_config_value())
        }),
        cancellation: CancellationToken::new(),
    };
    let result = run_sterile_copy(&options)?;
    let report = assemble(args.out.as_deref(), args, &safety_mode, &result);

    if format.is_json() {
        output::emit_json("sanitize", &report)?;
    } else {
        print_human(&report);
    }

    Ok(if report.summary.errors > 0 {
        Outcome::Incomplete
    } else {
        Outcome::Success
    })
}

/// A scratch directory beside the archive rather than in the system temp folder.
///
/// Two reasons, both about what ends up recorded: `STERILE_COPY_REPORT.json` — which is
/// packed *into* the archive — writes the destination path it was given, and a system
/// temp path there is a dead `…\Temp\.tmpA1b2` a recipient can make nothing of. Beside
/// the archive it is at least an intelligible sibling. It also puts the copy on the same
/// filesystem as the archive, so packing never crosses a volume.
fn scratch_beside_archive(args: &SanitizeArgs) -> Result<tempfile::TempDir> {
    let parent = args
        .archive
        .as_ref()
        .and_then(|path| path.parent())
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(std::env::temp_dir, std::path::Path::to_path_buf);

    std::fs::create_dir_all(&parent).map_err(|source| CliError::Read {
        path: parent.clone(),
        source,
    })?;
    tempfile::Builder::new()
        .prefix("codepack-sterile-")
        .tempdir_in(&parent)
        .map_err(|source| CliError::Read {
            path: parent,
            source,
        })
}

fn assemble(
    destination: Option<&std::path::Path>,
    args: &SanitizeArgs,
    safety_mode: &str,
    result: &SterileCopyReport,
) -> SanitizeReport {
    SanitizeReport {
        source: args.source.display().to_string(),
        destination: destination.map(|path| path.display().to_string()),
        safety_mode: safety_mode.to_string(),
        archive: result.archive.as_ref().map(|archive| ArchiveInfo {
            path: archive.path.display().to_string(),
            format: archive.format.as_str(),
            file_count: archive.file_count,
            bytes: archive.bytes,
            bytes_human: codepack_tokens::format_bytes(archive.bytes),
        }),
        summary: Summary {
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
            .map(|(path, outcome)| ReportedFile {
                path: path.to_string_lossy().replace('\\', "/"),
                outcome: outcome.label(),
                detail: detail_of(outcome),
            })
            .collect(),
    }
}

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

fn print_human(report: &SanitizeReport) {
    output::line(format!("Source:      {}", report.source));
    if let Some(destination) = &report.destination {
        output::line(format!("Destination: {destination}"));
    }
    if let Some(archive) = &report.archive {
        output::line(format!(
            "Archive:     {} [{}] ({} file(s), {})",
            archive.path, archive.format, archive.file_count, archive.bytes_human
        ));
    }
    output::line(format!("Safety mode: {}", report.safety_mode));
    output::line("");
    output::line(format!(
        "{} file(s): {} stripped and formatted, {} stripped only (no formatter found), \
         {} unsupported language, {} sensitive/redacted, {} error(s)",
        report.summary.total_files,
        report.summary.stripped_and_formatted,
        report.summary.stripped_only_no_formatter_found,
        report.summary.skipped_unsupported_language,
        report.summary.skipped_sensitive_or_redacted,
        report.summary.errors
    ));

    if report.summary.errors > 0 {
        output::line("");
        output::line("Errors:");
        for file in &report.files {
            if file.outcome == "error" {
                output::line(format!(
                    "  {} — {}",
                    file.path,
                    file.detail.as_deref().unwrap_or("")
                ));
            }
        }
    }
}
