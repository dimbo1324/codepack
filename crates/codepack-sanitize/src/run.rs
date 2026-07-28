//! [`run_sterile_copy`]: the standalone pipeline, per file —
//!
//! scanner plan (safe file selection) → safety-skip predicate → read content → redact
//! secrets (invariant I3, before anything is written) → strip comments via tree-sitter →
//! format via a `PATH` tool if one is found → write to `destination_root`, mirroring the
//! relative path from `source_root`.
//!
//! This never goes through `codepack-engine`'s 8-step pipeline (it is not one of its
//! steps, does not touch the ZIP/archive contract, and produces none of the existing
//! ~30 report artifacts) — see `docs/decisions/open-questions.md`, 2026-07-28.

use std::path::{Path, PathBuf};

use rayon::prelude::*;

use codepack_core::CancellationToken;
use codepack_scanner::{
    ExportIgnoreRules, ScanOptions, build_export_plan, looks_binary, should_consider_text_file,
};
use codepack_security::{redact_secrets, should_skip_file_for_safety};

use crate::error::{Result, SanitizeError};
use crate::format::format_source;
use crate::language::detect_language;
use crate::options::{FileOutcome, SterileCopyOptions, SterileCopyReport, SterileCopySummary};
use crate::report::write_report;
use crate::strip::strip_comments;

/// Runs one sterile-copy pass and writes `STERILE_COPY_REPORT.json`/`.md` into the
/// destination folder alongside the copied files.
pub fn run_sterile_copy(options: &SterileCopyOptions) -> Result<SterileCopyReport> {
    let (canonical_source, canonical_destination) =
        validate_destination(&options.source_root, &options.destination_root)?;

    let scan_options = ScanOptions {
        safe_export_mode: options.safety_mode.clone(),
        ..ScanOptions::default()
    };
    let export_rules = ExportIgnoreRules::from_project_and_config(&canonical_source, &scan_options);
    let safety_mode = options.safety_mode.clone();
    let safety = move |relative_path: &Path| -> Option<(String, String)> {
        let decision = should_skip_file_for_safety(relative_path, &safety_mode);
        decision
            .skip
            .then_some((decision.reason, decision.severity))
    };

    let plan = build_export_plan(
        &canonical_source,
        &scan_options,
        &export_rules,
        &safety,
        &options.cancellation,
    )
    .map_err(|error| match error {
        codepack_scanner::ScannerError::Cancelled => SanitizeError::Cancelled,
        other => SanitizeError::Scanner(other),
    })?;

    let mut per_file: Vec<(PathBuf, FileOutcome)> = plan
        .sensitive_warnings()
        .iter()
        .map(|planned| {
            (
                to_relative_path(&planned.relative_path),
                FileOutcome::SkippedSensitiveOrRedacted,
            )
        })
        .collect();

    let cancel = options.cancellation.clone();
    let source_for_workers = canonical_source.clone();
    let destination_for_workers = canonical_destination.clone();
    let processed: Vec<(PathBuf, FileOutcome)> = plan
        .included_files
        .par_iter()
        .map(|planned| {
            let relative = to_relative_path(&planned.relative_path);
            let outcome = process_file(
                &source_for_workers,
                &destination_for_workers,
                &relative,
                &cancel,
            );
            (relative, outcome)
        })
        .collect();
    per_file.extend(processed);

    let summary = SterileCopySummary::from_outcomes(per_file.iter().map(|(_, outcome)| outcome));
    let report = SterileCopyReport { per_file, summary };

    write_report(
        &canonical_destination,
        &options.source_root,
        &options.safety_mode,
        &report,
    )?;

    Ok(report)
}

/// Invariant analogous to I2: a sterile copy must never be written into, or as an
/// ancestor of, the project it reads from. Compared after canonicalization (both sides
/// must be real, existing paths) — the same pattern `codepack-cli`'s `--out` guard uses
/// for the export pipeline's own I2.
fn validate_destination(source_root: &Path, destination_root: &Path) -> Result<(PathBuf, PathBuf)> {
    if !source_root.is_dir() {
        return Err(SanitizeError::SourceNotADirectory {
            path: source_root.to_path_buf(),
        });
    }
    std::fs::create_dir_all(destination_root).map_err(|source| SanitizeError::Write {
        path: destination_root.to_path_buf(),
        source,
    })?;

    let canonical_source = canonicalize(source_root)?;
    let canonical_destination = canonicalize(destination_root)?;
    if canonical_destination.starts_with(&canonical_source) {
        return Err(SanitizeError::DestinationInsideSource {
            source_root: canonical_source,
            destination: canonical_destination,
        });
    }
    Ok((canonical_source, canonical_destination))
}

fn canonicalize(path: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(path).map_err(|source| SanitizeError::Read {
        path: path.to_path_buf(),
        source,
    })
}

/// The plan stores backslash-joined relative paths regardless of platform (a documented
/// contract of `codepack_scanner::ExportPlan`) — rebuild a real path component-by-
/// component rather than handing the string straight to `Path::new`.
fn to_relative_path(relative: &str) -> PathBuf {
    relative.split('\\').collect()
}

fn process_file(
    source_root: &Path,
    destination_root: &Path,
    relative: &Path,
    cancel: &CancellationToken,
) -> FileOutcome {
    if cancel.is_cancelled() {
        return FileOutcome::Error {
            message: "cancelled".to_string(),
        };
    }

    let source_path = source_root.join(relative);
    let raw = match std::fs::read(&source_path) {
        Ok(bytes) => bytes,
        Err(source) => {
            return FileOutcome::Error {
                message: format!("failed to read {}: {source}", source_path.display()),
            };
        }
    };

    if !should_consider_text_file(relative) || looks_binary(&raw) {
        return finish(
            destination_root,
            relative,
            &raw,
            FileOutcome::SkippedUnsupportedLanguage {
                reason: "binary file (not text)".to_string(),
            },
        );
    }

    let text = match String::from_utf8(raw) {
        Ok(text) => text,
        Err(error) => {
            return finish(
                destination_root,
                relative,
                &error.into_bytes(),
                FileOutcome::Error {
                    message: "file content is not valid UTF-8; copied through unmodified"
                        .to_string(),
                },
            );
        }
    };

    // Invariant I3: a secret must never reach an artifact unredacted — applied before
    // any stripping/formatting, and on every path out of this function from here on,
    // including the unsupported-language and parse-error fallbacks below.
    let redacted = redact_secrets(&text);

    let Some(language) = detect_language(relative) else {
        return finish(
            destination_root,
            relative,
            redacted.as_bytes(),
            FileOutcome::SkippedUnsupportedLanguage {
                reason: "not one of the Batch 1 languages (docs/decisions/open-questions.md, Q24)"
                    .to_string(),
            },
        );
    };

    let Some(stripped) = strip_comments(language, &redacted) else {
        return finish(
            destination_root,
            relative,
            redacted.as_bytes(),
            FileOutcome::Error {
                message: "tree-sitter could not fully parse this file (syntax error); comments \
                          were not stripped to avoid corrupting the code"
                    .to_string(),
            },
        );
    };

    let file_name = relative
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    match format_source(language, &file_name, &stripped) {
        Some((formatted, formatter)) => finish(
            destination_root,
            relative,
            formatted.as_bytes(),
            FileOutcome::StrippedAndFormatted {
                language: language.label().to_string(),
                formatter,
            },
        ),
        None => finish(
            destination_root,
            relative,
            stripped.as_bytes(),
            FileOutcome::StrippedOnlyNoFormatterFound {
                language: language.label().to_string(),
            },
        ),
    }
}

fn finish(
    destination_root: &Path,
    relative: &Path,
    bytes: &[u8],
    outcome: FileOutcome,
) -> FileOutcome {
    let destination_path = destination_root.join(relative);
    if let Some(parent) = destination_path.parent()
        && let Err(source) = std::fs::create_dir_all(parent)
    {
        return FileOutcome::Error {
            message: format!("failed to create {}: {source}", parent.display()),
        };
    }
    match std::fs::write(&destination_path, bytes) {
        Ok(()) => outcome,
        Err(source) => FileOutcome::Error {
            message: format!("failed to write {}: {source}", destination_path.display()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn options(source: &Path, destination: &Path) -> SterileCopyOptions {
        SterileCopyOptions {
            source_root: source.to_path_buf(),
            destination_root: destination.to_path_buf(),
            safety_mode: "safe".to_string(),
            cancellation: CancellationToken::new(),
        }
    }

    #[test]
    fn destination_equal_to_source_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let error = run_sterile_copy(&options(dir.path(), dir.path())).unwrap_err();
        assert!(matches!(
            error,
            SanitizeError::DestinationInsideSource { .. }
        ));
    }

    #[test]
    fn destination_nested_inside_source_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("out");
        let error = run_sterile_copy(&options(dir.path(), &nested)).unwrap_err();
        assert!(matches!(
            error,
            SanitizeError::DestinationInsideSource { .. }
        ));
    }

    #[test]
    fn a_supported_language_file_is_stripped_and_copied() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(source.path().join("main.py"), "# a comment\nx = 1\n").unwrap();

        let report = run_sterile_copy(&options(source.path(), destination.path())).unwrap();
        assert_eq!(report.summary.total_files, 1);

        let written = fs::read_to_string(destination.path().join("main.py")).unwrap();
        assert!(!written.contains("a comment"));
    }

    #[test]
    fn an_unsupported_language_file_is_copied_through_and_named_honestly() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(source.path().join("main.dart"), "// hi\nvoid main() {}\n").unwrap();

        let report = run_sterile_copy(&options(source.path(), destination.path())).unwrap();
        let (_, outcome) = &report.per_file[0];
        assert!(matches!(
            outcome,
            FileOutcome::SkippedUnsupportedLanguage { .. }
        ));
        // Comments are not stripped for an unsupported language — copied through as-is
        // (after redaction, which this fixture has nothing for).
        let written = fs::read_to_string(destination.path().join("main.dart")).unwrap();
        assert!(written.contains("// hi"));
    }

    #[test]
    fn a_planted_secret_is_redacted_before_it_reaches_the_destination_copy() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(
            source.path().join("app.py"),
            "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n# a comment\n",
        )
        .unwrap();

        run_sterile_copy(&options(source.path(), destination.path())).unwrap();

        let written = fs::read_to_string(destination.path().join("app.py")).unwrap();
        assert!(!written.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn a_file_safety_mode_would_exclude_from_export_never_appears_in_the_destination() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(source.path().join(".env"), "SECRET=hunter2\n").unwrap();
        fs::write(source.path().join("main.py"), "x = 1\n").unwrap();

        let report = run_sterile_copy(&options(source.path(), destination.path())).unwrap();

        assert!(!destination.path().join(".env").exists());
        let env_entry = report
            .per_file
            .iter()
            .find(|(path, _)| path == Path::new(".env"))
            .expect(".env must still be named in the report, not silently dropped");
        assert_eq!(env_entry.1, FileOutcome::SkippedSensitiveOrRedacted);
    }

    #[test]
    fn a_run_cancelled_before_the_plan_is_even_built_is_a_clean_error() {
        // Cancellation checked between phases: the scanner refuses to hand back a plan
        // at all once the token is set before it starts.
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(source.path().join("main.py"), "x = 1\n").unwrap();

        let opts = options(source.path(), destination.path());
        opts.cancellation.cancel();
        let error = run_sterile_copy(&opts).unwrap_err();
        assert!(matches!(error, SanitizeError::Cancelled));
    }

    #[test]
    fn cancellation_is_checked_inside_the_per_file_loop_not_only_between_phases() {
        // Exercises `process_file`'s own check directly (`.ai/project/12-domain-
        // rules.md`'s standing rule for long operations): a plan can finish building
        // and then the token can flip mid-run, and a file reached after that point must
        // never be read, stripped, formatted or written — only marked cancelled.
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(source.path().join("main.py"), "x = 1\n").unwrap();

        let cancel = CancellationToken::new();
        cancel.cancel();
        let outcome = process_file(
            source.path(),
            destination.path(),
            Path::new("main.py"),
            &cancel,
        );
        assert!(matches!(outcome, FileOutcome::Error { message } if message == "cancelled"));
        assert!(!destination.path().join("main.py").exists());
    }

    #[test]
    fn a_syntactically_broken_file_is_copied_unmodified_and_reported_as_an_error() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(source.path().join("broken.rs"), "fn main( {\n").unwrap();

        let report = run_sterile_copy(&options(source.path(), destination.path())).unwrap();
        let (_, outcome) = &report.per_file[0];
        assert!(matches!(outcome, FileOutcome::Error { .. }));

        let written = fs::read_to_string(destination.path().join("broken.rs")).unwrap();
        assert_eq!(written, "fn main( {\n");
    }

    #[test]
    fn the_report_artifact_is_written_into_the_destination() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::write(source.path().join("main.py"), "x = 1\n").unwrap();

        run_sterile_copy(&options(source.path(), destination.path())).unwrap();

        assert!(
            destination
                .path()
                .join("STERILE_COPY_REPORT.json")
                .is_file()
        );
        assert!(destination.path().join("STERILE_COPY_REPORT.md").is_file());
    }
}
