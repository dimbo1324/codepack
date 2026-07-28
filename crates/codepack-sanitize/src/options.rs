//! Public request/response shapes for [`crate::run_sterile_copy`].

use std::path::PathBuf;

use codepack_core::CancellationToken;

/// One sterile-copy run's inputs.
#[derive(Debug, Clone)]
pub struct SterileCopyOptions {
    pub source_root: PathBuf,
    pub destination_root: PathBuf,
    /// `"safe"` / `"balanced"` / `"full"` — the same three modes
    /// `codepack_security::should_skip_file_for_safety` already accepts; an invalid
    /// value falls back to `"safe"`, exactly like the rest of the product.
    pub safety_mode: String,
    pub cancellation: CancellationToken,
}

/// What happened to one file. Every included file gets exactly one of these — never
/// silently dropped, per `docs/decisions/open-questions.md` (2026-07-28).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOutcome {
    /// Comments were stripped and an external formatter on `PATH` reformatted the
    /// result.
    StrippedAndFormatted { language: String, formatter: String },
    /// Comments were stripped; no formatter for this language was found on `PATH` (or
    /// every candidate failed), so the stripped content was written as-is.
    StrippedOnlyNoFormatterFound { language: String },
    /// Not one of the Batch 1 languages (`crate::language`), or not recognized as a
    /// text file at all — copied through unmodified (after redaction) rather than
    /// dropped.
    SkippedUnsupportedLanguage { reason: String },
    /// Excluded by the same safety predicate a normal export would exclude it with
    /// (`codepack_security::should_skip_file_for_safety`) — never appears in the
    /// destination folder at all.
    SkippedSensitiveOrRedacted,
    /// Copied through unmodified (after redaction) because tree-sitter could not fully
    /// parse it (an `ERROR` node was present) or its bytes were not valid UTF-8 — the
    /// fail-safe default from `docs/decisions/open-questions.md` Q24: never strip under
    /// uncertain parse conditions.
    Error { message: String },
}

impl FileOutcome {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StrippedAndFormatted { .. } => "stripped_and_formatted",
            Self::StrippedOnlyNoFormatterFound { .. } => "stripped_only_no_formatter_found",
            Self::SkippedUnsupportedLanguage { .. } => "skipped_unsupported_language",
            Self::SkippedSensitiveOrRedacted => "skipped_sensitive_or_redacted",
            Self::Error { .. } => "error",
        }
    }
}

/// Aggregate counts over every file [`FileOutcome`] was computed for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SterileCopySummary {
    pub total_files: usize,
    pub stripped_and_formatted: usize,
    pub stripped_only_no_formatter_found: usize,
    pub skipped_unsupported_language: usize,
    pub skipped_sensitive_or_redacted: usize,
    pub errors: usize,
}

impl SterileCopySummary {
    pub fn from_outcomes<'a>(outcomes: impl IntoIterator<Item = &'a FileOutcome>) -> Self {
        let mut summary = Self::default();
        for outcome in outcomes {
            summary.total_files += 1;
            match outcome {
                FileOutcome::StrippedAndFormatted { .. } => summary.stripped_and_formatted += 1,
                FileOutcome::StrippedOnlyNoFormatterFound { .. } => {
                    summary.stripped_only_no_formatter_found += 1;
                }
                FileOutcome::SkippedUnsupportedLanguage { .. } => {
                    summary.skipped_unsupported_language += 1;
                }
                FileOutcome::SkippedSensitiveOrRedacted => {
                    summary.skipped_sensitive_or_redacted += 1;
                }
                FileOutcome::Error { .. } => summary.errors += 1,
            }
        }
        summary
    }
}

/// One run's full result: every file's disposition plus the aggregate summary written
/// into `STERILE_COPY_REPORT.md`/`.json`.
#[derive(Debug, Clone)]
pub struct SterileCopyReport {
    pub per_file: Vec<(PathBuf, FileOutcome)>,
    pub summary: SterileCopySummary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_every_variant_once() {
        let outcomes = vec![
            FileOutcome::StrippedAndFormatted {
                language: "Rust".to_string(),
                formatter: "rustfmt".to_string(),
            },
            FileOutcome::StrippedOnlyNoFormatterFound {
                language: "Ruby".to_string(),
            },
            FileOutcome::SkippedUnsupportedLanguage {
                reason: "Dart (Batch 2, not yet supported)".to_string(),
            },
            FileOutcome::SkippedSensitiveOrRedacted,
            FileOutcome::Error {
                message: "parse error".to_string(),
            },
        ];
        let summary = SterileCopySummary::from_outcomes(&outcomes);
        assert_eq!(summary.total_files, 5);
        assert_eq!(summary.stripped_and_formatted, 1);
        assert_eq!(summary.stripped_only_no_formatter_found, 1);
        assert_eq!(summary.skipped_unsupported_language, 1);
        assert_eq!(summary.skipped_sensitive_or_redacted, 1);
        assert_eq!(summary.errors, 1);
    }
}
