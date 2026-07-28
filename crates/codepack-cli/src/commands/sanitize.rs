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
use crate::error::Result;
use crate::exit::Outcome;
use crate::output::{self, Format};

#[derive(Debug, Serialize)]
pub(crate) struct SanitizeReport {
    pub source: String,
    pub destination: String,
    pub safety_mode: String,
    pub summary: Summary,
    pub files: Vec<ReportedFile>,
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

    let options = SterileCopyOptions {
        source_root: args.source.clone(),
        destination_root: args.out.clone(),
        safety_mode: safety_mode.clone(),
        cancellation: CancellationToken::new(),
    };
    let result = run_sterile_copy(&options)?;
    let report = assemble(args, &safety_mode, &result);

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

fn assemble(args: &SanitizeArgs, safety_mode: &str, result: &SterileCopyReport) -> SanitizeReport {
    SanitizeReport {
        source: args.source.display().to_string(),
        destination: args.out.display().to_string(),
        safety_mode: safety_mode.to_string(),
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
    output::line(format!("Destination: {}", report.destination));
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
