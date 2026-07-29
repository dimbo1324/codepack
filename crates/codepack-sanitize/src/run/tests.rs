//! Tests for the parent module. Split out when the file passed the ~600-line
//! limit in `.ai/project/12-domain-rules.md`; the code itself is unchanged, and
//! `use super::*` still reaches exactly what it reached inline.

use super::*;
use std::fs;

fn options(source: &Path, destination: &Path) -> SterileCopyOptions {
    SterileCopyOptions {
        source_root: source.to_path_buf(),
        destination_root: destination.to_path_buf(),
        safety_mode: "safe".to_string(),
        archive_path: None,
        archive_format: None,
        cancellation: CancellationToken::new(),
    }
}

fn source_with_code() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("main.rs"),
        "// a comment\nfn main() { println!(\"hi\"); }\n",
    )
    .unwrap();
    dir
}

#[test]
fn no_archive_path_leaves_todays_behaviour_untouched() {
    let source = source_with_code();
    let destination = tempfile::tempdir().unwrap();

    let report = run_sterile_copy(&options(source.path(), destination.path())).unwrap();

    assert!(report.archive.is_none());
    assert!(destination.path().join("main.rs").is_file());
    assert!(
        !destination.path().join("main.rs.7z").exists(),
        "nothing may be packed unless an archive was asked for"
    );
}

#[test]
fn an_archive_path_packs_the_finished_copy_including_its_report() {
    let source = source_with_code();
    let destination = tempfile::tempdir().unwrap();
    let archive_dir = tempfile::tempdir().unwrap();
    let archive_path = archive_dir.path().join("sterile.7z");

    let mut options = options(source.path(), destination.path());
    options.archive_path = Some(archive_path.clone());
    let report = run_sterile_copy(&options).unwrap();

    let archive = report.archive.expect("an archive was asked for");
    assert_eq!(archive.path, archive_path);
    assert!(archive.bytes > 0);
    assert!(archive_path.is_file());

    // The report must be *inside* the archive: a recipient holding only the `.7z`
    // otherwise has the code with no account of what was stripped or redacted.
    let mut names = Vec::new();
    let mut reader = sevenz_rust2::ArchiveReader::open(&archive_path, Default::default()).unwrap();
    reader
        .for_each_entries(|entry, _| {
            names.push(entry.name.clone());
            Ok(true)
        })
        .unwrap();
    assert!(names.iter().any(|name| name == "main.rs"), "{names:?}");
    assert!(
        names
            .iter()
            .any(|name| name.contains("STERILE_COPY_REPORT")),
        "{names:?}"
    );
}

#[test]
fn a_stray_file_already_in_the_destination_never_reaches_the_archive() {
    // Regression, found by review 2026-07-29. The destination is not required to be
    // empty, so walking it to build the archive swept in files that passed neither
    // redaction nor the safety filter and appear in no report — into an archive
    // whose whole promise is that everything inside was screened (invariant I3).
    let source = source_with_code();
    let destination = tempfile::tempdir().unwrap();
    fs::write(
        destination.path().join("UNSCREENED.txt"),
        "never went through redaction\n",
    )
    .unwrap();
    let archive_dir = tempfile::tempdir().unwrap();
    let archive_path = archive_dir.path().join("sterile.7z");

    let mut options = options(source.path(), destination.path());
    options.archive_path = Some(archive_path.clone());
    let report = run_sterile_copy(&options).unwrap();

    let mut names = Vec::new();
    let mut reader = sevenz_rust2::ArchiveReader::open(&archive_path, Default::default()).unwrap();
    reader
        .for_each_entries(|entry, _| {
            names.push(entry.name.clone());
            Ok(true)
        })
        .unwrap();

    assert!(
        !names.iter().any(|name| name.contains("UNSCREENED")),
        "an unscreened file reached the archive: {names:?}"
    );
    assert_eq!(
        report.archive.unwrap().file_count,
        names.len(),
        "the reported count must match what is actually inside"
    );
}

#[test]
fn a_previous_archive_survives_a_cancelled_rerun() {
    let source = source_with_code();
    let destination = tempfile::tempdir().unwrap();
    let archive_dir = tempfile::tempdir().unwrap();
    let archive_path = archive_dir.path().join("weekly.7z");
    fs::write(&archive_path, b"last week's archive").unwrap();

    let mut options = options(source.path(), destination.path());
    options.archive_path = Some(archive_path.clone());
    options.cancellation.cancel();
    assert!(run_sterile_copy(&options).is_err());

    assert_eq!(fs::read(&archive_path).unwrap(), b"last week's archive");
}

#[test]
fn an_archive_path_inside_the_source_is_rejected_before_anything_is_written() {
    let source = source_with_code();
    let destination = tempfile::tempdir().unwrap();
    let archive_path = source.path().join("sterile.7z");

    let mut options = options(source.path(), destination.path());
    options.archive_path = Some(archive_path.clone());
    let error = run_sterile_copy(&options).unwrap_err();

    assert!(matches!(error, SanitizeError::ArchiveInsideSource { .. }));
    assert!(
        !archive_path.exists(),
        "the source project must be left exactly as it was found (invariant I2)"
    );
}

#[test]
fn a_cancelled_run_produces_no_archive() {
    let source = source_with_code();
    let destination = tempfile::tempdir().unwrap();
    let archive_dir = tempfile::tempdir().unwrap();
    let archive_path = archive_dir.path().join("sterile.7z");

    let mut options = options(source.path(), destination.path());
    options.archive_path = Some(archive_path.clone());
    options.cancellation.cancel();

    assert!(run_sterile_copy(&options).is_err());
    assert!(!archive_path.exists());
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
fn a_rejected_overlapping_destination_is_never_created_inside_the_source() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("out").join("nested");
    let error = run_sterile_copy(&options(dir.path(), &nested)).unwrap_err();
    assert!(matches!(
        error,
        SanitizeError::DestinationInsideSource { .. }
    ));
    assert!(
        !dir.path().join("out").exists(),
        "the overlap guard must reject before creating anything on disk"
    );
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
