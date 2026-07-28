//! `STERILE_COPY_REPORT.json`/`.md` — a brand-new artifact contract (not an existing
//! ~30-report format, invariant I5 does not apply to it), so it carries `schema_version`
//! from its very first version rather than growing one retroactively (owner decision,
//! `docs/decisions/open-questions.md`, 2026-07-28).

use std::path::Path;

use serde::Serialize;

use crate::error::{Result, SanitizeError};
use crate::options::{FileOutcome, SterileCopyReport, SterileCopySummary};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct ReportArtifact {
    schema_version: u32,
    generated_at: String,
    source_root: String,
    destination_root: String,
    safety_mode: String,
    summary: SummaryArtifact,
    files: Vec<FileArtifact>,
}

#[derive(Debug, Serialize)]
struct SummaryArtifact {
    total_files: usize,
    stripped_and_formatted: usize,
    stripped_only_no_formatter_found: usize,
    skipped_unsupported_language: usize,
    skipped_sensitive_or_redacted: usize,
    errors: usize,
}

impl From<SterileCopySummary> for SummaryArtifact {
    fn from(summary: SterileCopySummary) -> Self {
        Self {
            total_files: summary.total_files,
            stripped_and_formatted: summary.stripped_and_formatted,
            stripped_only_no_formatter_found: summary.stripped_only_no_formatter_found,
            skipped_unsupported_language: summary.skipped_unsupported_language,
            skipped_sensitive_or_redacted: summary.skipped_sensitive_or_redacted,
            errors: summary.errors,
        }
    }
}

#[derive(Debug, Serialize)]
struct FileArtifact {
    relative_path: String,
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formatter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

fn to_artifact(relative_path: &Path, outcome: &FileOutcome) -> FileArtifact {
    let (language, formatter, reason) = match outcome {
        FileOutcome::StrippedAndFormatted {
            language,
            formatter,
        } => (Some(language.clone()), Some(formatter.clone()), None),
        FileOutcome::StrippedOnlyNoFormatterFound { language } => {
            (Some(language.clone()), None, None)
        }
        FileOutcome::SkippedUnsupportedLanguage { reason } => (None, None, Some(reason.clone())),
        FileOutcome::SkippedSensitiveOrRedacted => (None, None, None),
        FileOutcome::Error { message } => (None, None, Some(message.clone())),
    };
    FileArtifact {
        relative_path: relative_path.to_string_lossy().replace('\\', "/"),
        outcome: outcome.label(),
        language,
        formatter,
        reason,
    }
}

/// Writes both `STERILE_COPY_REPORT.json` and `.md` into `destination_root`.
pub(crate) fn write_report(
    destination_root: &Path,
    source_root: &Path,
    safety_mode: &str,
    report: &SterileCopyReport,
) -> Result<()> {
    let artifact = ReportArtifact {
        schema_version: SCHEMA_VERSION,
        generated_at: codepack_core::time::now_human_utc(),
        source_root: source_root.display().to_string(),
        destination_root: destination_root.display().to_string(),
        safety_mode: safety_mode.to_string(),
        summary: report.summary.into(),
        files: report
            .per_file
            .iter()
            .map(|(path, outcome)| to_artifact(path, outcome))
            .collect(),
    };

    let json_path = destination_root.join("STERILE_COPY_REPORT.json");
    let json = serde_json::to_string_pretty(&artifact).map_err(SanitizeError::Serialize)?;
    std::fs::write(&json_path, json).map_err(|source| SanitizeError::Write {
        path: json_path.clone(),
        source,
    })?;

    let markdown = render_markdown(&artifact);
    let markdown_path = destination_root.join("STERILE_COPY_REPORT.md");
    std::fs::write(&markdown_path, markdown).map_err(|source| SanitizeError::Write {
        path: markdown_path.clone(),
        source,
    })?;

    Ok(())
}

fn render_markdown(artifact: &ReportArtifact) -> String {
    let mut out = String::new();
    out.push_str("# Sterile Copy Report\n\n");
    out.push_str(&format!("- schema_version: {}\n", artifact.schema_version));
    out.push_str(&format!("- generated_at: {}\n", artifact.generated_at));
    out.push_str(&format!("- source: {}\n", artifact.source_root));
    out.push_str(&format!("- destination: {}\n", artifact.destination_root));
    out.push_str(&format!("- safety_mode: {}\n\n", artifact.safety_mode));

    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "- total files: {}\n",
        artifact.summary.total_files
    ));
    out.push_str(&format!(
        "- stripped and formatted: {}\n",
        artifact.summary.stripped_and_formatted
    ));
    out.push_str(&format!(
        "- stripped only (no formatter found): {}\n",
        artifact.summary.stripped_only_no_formatter_found
    ));
    out.push_str(&format!(
        "- skipped (unsupported language): {}\n",
        artifact.summary.skipped_unsupported_language
    ));
    out.push_str(&format!(
        "- skipped (sensitive/redacted): {}\n",
        artifact.summary.skipped_sensitive_or_redacted
    ));
    out.push_str(&format!("- errors: {}\n\n", artifact.summary.errors));

    out.push_str("## Files\n\n");
    out.push_str("| Path | Outcome | Detail |\n|---|---|---|\n");
    for file in &artifact.files {
        let detail = file
            .formatter
            .as_deref()
            .map(|formatter| format!("{} → {formatter}", file.language.as_deref().unwrap_or("")))
            .or_else(|| file.language.clone())
            .or_else(|| file.reason.clone())
            .unwrap_or_default();
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            file.relative_path, file.outcome, detail
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn writes_both_artifact_files_with_the_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let report = SterileCopyReport {
            per_file: vec![(
                PathBuf::from("main.rs"),
                FileOutcome::StrippedAndFormatted {
                    language: "Rust".to_string(),
                    formatter: "rustfmt".to_string(),
                },
            )],
            summary: SterileCopySummary::from_outcomes(&[FileOutcome::StrippedAndFormatted {
                language: "Rust".to_string(),
                formatter: "rustfmt".to_string(),
            }]),
        };

        write_report(dir.path(), source.path(), "safe", &report).unwrap();

        let json = std::fs::read_to_string(dir.path().join("STERILE_COPY_REPORT.json")).unwrap();
        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"rustfmt\""));

        let markdown = std::fs::read_to_string(dir.path().join("STERILE_COPY_REPORT.md")).unwrap();
        assert!(markdown.contains("schema_version: 1"));
        assert!(markdown.contains("main.rs"));
    }
}
