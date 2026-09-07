use super::*;
use codepack_core::AppPaths;
use codepack_storage::NewExportRun;

/// A fresh, isolated `AppPaths` under a tempdir, so these tests never touch whoever runs
/// `cargo test`'s real settings directory or history database (audit 2026-09-07,
/// S-7/T-2). Every command in this module now validates its `result_path` against the
/// history database (audit S-1), so a test calling one of them needs a database of its
/// own to record the bundle in — not the developer's real one.
fn isolated_paths(root: &std::path::Path) -> AppPaths {
    AppPaths::for_root(root)
}

/// Records one export run whose `result_path` is `bundle_path`, so
/// `ValidatedResultPath::resolve` — which every command in this module now goes through
/// — accepts it.
fn record_a_run_with_result_path(paths: &AppPaths, bundle_path: &std::path::Path) {
    let mut connection = crate::commands::open_database_at(paths).unwrap();
    let project =
        codepack_storage::find_or_create_project(&connection, "/tmp/project", "project", None)
            .unwrap();
    codepack_storage::record_export_run(
        &mut connection,
        NewExportRun {
            project_id: project,
            started_at: 10,
            finished_at: Some(11),
            profile: Some("full".to_string()),
            safe_mode: Some("safe".to_string()),
            diff_mode: Some("all".to_string()),
            files_copied: Some(1),
            bytes_total: Some(10),
            tokens_est: Some(1),
            redacted_count: None,
            cancelled: false,
            result_path: Some(bundle_path.display().to_string()),
        },
        &[],
        &[],
        &[],
        None,
    )
    .unwrap();
}

/// A bundle directory as it looks once extracted: profile at the root, dashboard
/// under the reports tree.
fn extracted_bundle(dir: &std::path::Path) {
    std::fs::write(
        dir.join("PROJECT_PROFILE.json"),
        r#"{
            "project_type": "fullstack",
            "detected_stack": ["Rust", "TypeScript"],
            "risk_level": "medium",
            "risk_reasons": ["secrets found"],
            "counts": { "files": 42, "folders": 7, "total_size_bytes": 1024 }
        }"#,
    )
    .unwrap();
    let reports = dir.join("reports").join("insights");
    std::fs::create_dir_all(&reports).unwrap();
    std::fs::write(reports.join("REPORT_DASHBOARD.html"), "<html></html>").unwrap();
}

#[test]
fn a_profile_is_read_from_an_already_extracted_bundle_directory() {
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    extracted_bundle(dir.path());
    record_a_run_with_result_path(&paths, dir.path());

    let summary = read_project_profile_at(&paths, &dir.path().display().to_string()).unwrap();
    assert_eq!(summary.project_type, "fullstack");
    assert_eq!(summary.detected_stack, vec!["Rust", "TypeScript"]);
    assert_eq!(summary.risk_level, "medium");
    assert_eq!(summary.risk_reasons, vec!["secrets found"]);
    assert_eq!(summary.files, 42);
    assert_eq!(summary.folders, 7);
    assert_eq!(summary.total_size_bytes, 1024);
}

#[test]
fn a_bundle_that_nests_everything_under_the_project_name_is_still_searched() {
    // An export with `include_project_in_zip` puts the reports one level down.
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("demo_export");
    std::fs::create_dir_all(&nested).unwrap();
    extracted_bundle(&nested);
    record_a_run_with_result_path(&paths, dir.path());

    let summary = read_project_profile_at(&paths, &dir.path().display().to_string()).unwrap();
    assert_eq!(summary.project_type, "fullstack");
}

#[test]
fn a_bundle_with_no_profile_explains_itself_rather_than_failing_opaquely() {
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    record_a_run_with_result_path(&paths, dir.path());

    let error = read_project_profile_at(&paths, &dir.path().display().to_string()).unwrap_err();
    assert!(
        error.message.contains("PROJECT_PROFILE.json"),
        "unhelpful message: {}",
        error.message
    );
}

#[test]
fn a_profile_missing_optional_fields_still_reads_rather_than_erroring() {
    // A bundle from an older version, or one whose analytics step was cut short.
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("PROJECT_PROFILE.json"), "{}").unwrap();
    record_a_run_with_result_path(&paths, dir.path());

    let summary = read_project_profile_at(&paths, &dir.path().display().to_string()).unwrap();
    assert_eq!(summary.project_type, "");
    assert!(summary.detected_stack.is_empty());
    assert_eq!(summary.files, 0);
}

#[test]
fn a_result_path_that_no_longer_exists_says_so() {
    // Archives get moved and deleted; history still remembers them. Deliberately *not*
    // recorded here: a path history never heard of and a path history heard of but that
    // vanished are two different messages, and this test is about neither of them — see
    // `a_path_no_run_produced_is_refused` for the first. What this covers is the
    // `canonicalize()` failure inside `ValidatedResultPath::resolve` itself, which is the
    // same failure whether or not the path was ever recorded.
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("gone.zip");
    let error = read_project_profile_at(&paths, &missing.display().to_string()).unwrap_err();
    assert!(
        error.message.contains("no longer where it was recorded"),
        "unhelpful message: {}",
        error.message
    );
}

/// The validation audit 2026-09-07 (S-1) found missing from this command entirely: a
/// directory nobody exported must not be read as if it were a bundle, however well
/// formed its contents look.
#[test]
fn a_bundle_directory_no_run_produced_is_refused() {
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    extracted_bundle(dir.path());
    // Deliberately not recorded.

    let error = read_project_profile_at(&paths, &dir.path().display().to_string())
        .expect_err("an unrecorded bundle directory must not be read");
    assert!(
        error
            .message
            .contains("not an export this installation produced"),
        "{}",
        error.message
    );
}

#[test]
fn a_real_archive_is_extracted_and_read_from_the_inside() {
    // The case the loose-file version of this command could never handle: the
    // pipeline deletes staging, so the profile exists only inside the ZIP.
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let source = tempfile::tempdir().unwrap();
    extracted_bundle(source.path());

    let out = tempfile::tempdir().unwrap();
    let archive_path = out.path().join("demo_export.zip");
    let file = std::fs::File::create(&archive_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    for relative in [
        "PROJECT_PROFILE.json",
        "reports/insights/REPORT_DASHBOARD.html",
    ] {
        writer.start_file(relative, options).unwrap();
        let native = source
            .path()
            .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        let bytes = std::fs::read(native).unwrap();
        std::io::Write::write_all(&mut writer, &bytes).unwrap();
    }
    writer.finish().unwrap();
    record_a_run_with_result_path(&paths, &archive_path);

    let summary = read_project_profile_at(&paths, &archive_path.display().to_string()).unwrap();
    assert_eq!(summary.project_type, "fullstack");
    assert_eq!(summary.files, 42);

    // Since audit No. 6 the extraction lands under the application's own data directory
    // rather than beside the archive: writing a folder next to somebody's file is a
    // liberty a command should not take. Where exactly is an implementation detail, so
    // what is asserted is that nothing was written beside the archive.
    assert!(
        !out.path().join("demo_export_extracted").exists(),
        "the archive's own directory must be left alone"
    );
}

#[test]
fn the_dashboard_is_found_under_the_reports_tree() {
    let dir = tempfile::tempdir().unwrap();
    extracted_bundle(dir.path());
    assert!(find_in_bundle(dir.path(), &["reports/insights/REPORT_DASHBOARD.html"]).is_some());
}

#[test]
fn a_bundle_with_no_dashboard_is_reported_rather_than_opened() {
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("PROJECT_PROFILE.json"), "{}").unwrap();
    record_a_run_with_result_path(&paths, dir.path());

    let error = open_bundle_report(
        &paths,
        &dir.path().display().to_string(),
        &[
            "reports/insights/REPORT_DASHBOARD.html",
            "REPORT_DASHBOARD.html",
        ],
        "this export contains no REPORT_DASHBOARD.html; the run may have been cancelled \
         before its reports were written",
    )
    .unwrap_err();
    assert!(
        error.message.contains("REPORT_DASHBOARD.html"),
        "unhelpful message: {}",
        error.message
    );
}

/// The validation audit 2026-09-07 (S-1) found missing from every report-opening command:
/// none of the four checked that `result_path` was ever an export this installation
/// produced before extracting and handing a file inside it to the OS opener.
#[test]
fn opening_a_report_from_an_unrecorded_bundle_is_refused() {
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    extracted_bundle(dir.path());
    // Deliberately not recorded.

    let error = open_bundle_report(
        &paths,
        &dir.path().display().to_string(),
        &[
            "reports/insights/REPORT_DASHBOARD.html",
            "REPORT_DASHBOARD.html",
        ],
        "this export contains no REPORT_DASHBOARD.html; the run may have been cancelled \
         before its reports were written",
    )
    .expect_err("an unrecorded bundle must not be opened");
    assert!(
        error
            .message
            .contains("not an export this installation produced"),
        "{}",
        error.message
    );
}

/// One test per S12 report-opening command, each pinning that it looks for its
/// own filename rather than accidentally reusing another command's message (the
/// risk `open_bundle_report`'s shared body introduces, that four separate
/// hand-written functions would not have had).
#[test]
fn a_bundle_with_no_overview_is_reported_rather_than_opened() {
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("PROJECT_PROFILE.json"), "{}").unwrap();
    record_a_run_with_result_path(&paths, dir.path());

    let error = open_bundle_report(
        &paths,
        &dir.path().display().to_string(),
        &[
            "reports/insights/PROJECT_OVERVIEW.html",
            "PROJECT_OVERVIEW.html",
        ],
        "this export contains no PROJECT_OVERVIEW.html; the run may have been cancelled \
         before its reports were written",
    )
    .unwrap_err();
    assert!(
        error.message.contains("PROJECT_OVERVIEW.html"),
        "unhelpful message: {}",
        error.message
    );
}

#[test]
fn a_bundle_with_no_onboarding_guide_is_reported_rather_than_opened() {
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("PROJECT_PROFILE.json"), "{}").unwrap();
    record_a_run_with_result_path(&paths, dir.path());

    let error = open_bundle_report(
        &paths,
        &dir.path().display().to_string(),
        &[
            "reports/insights/ONBOARDING_GUIDE.md",
            "ONBOARDING_GUIDE.md",
        ],
        "this export contains no ONBOARDING_GUIDE.md; the run may have been cancelled \
         before its reports were written",
    )
    .unwrap_err();
    assert!(
        error.message.contains("ONBOARDING_GUIDE.md"),
        "unhelpful message: {}",
        error.message
    );
}

#[test]
fn a_bundle_with_no_review_checklist_is_reported_rather_than_opened() {
    let history_root = tempfile::tempdir().unwrap();
    let paths = isolated_paths(history_root.path());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("PROJECT_PROFILE.json"), "{}").unwrap();
    record_a_run_with_result_path(&paths, dir.path());

    let error = open_bundle_report(
        &paths,
        &dir.path().display().to_string(),
        &[
            "reports/insights/REVIEW_CHECKLIST.md",
            "REVIEW_CHECKLIST.md",
        ],
        "this export contains no REVIEW_CHECKLIST.md; the run may have been cancelled \
         before its reports were written",
    )
    .unwrap_err();
    assert!(
        error.message.contains("REVIEW_CHECKLIST.md"),
        "unhelpful message: {}",
        error.message
    );
}

#[test]
fn each_s12_command_finds_its_own_file_in_an_extracted_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let reports = dir.path().join("reports").join("insights");
    std::fs::create_dir_all(&reports).unwrap();
    std::fs::write(reports.join("PROJECT_OVERVIEW.html"), "<html></html>").unwrap();
    std::fs::write(reports.join("ONBOARDING_GUIDE.md"), "# guide\n").unwrap();
    std::fs::write(reports.join("REVIEW_CHECKLIST.md"), "# checklist\n").unwrap();

    // Only the lookup is exercised here, not the OS-handler dispatch (which
    // open_dashboard's own tests do not exercise either, for the same reason:
    // it would launch a real application in the test process). A file found by
    // find_in_bundle is exactly the input open_bundle_report hands to the opener.
    let bundle_dir = dir.path();
    assert!(
        find_in_bundle(
            bundle_dir,
            &[
                "reports/insights/PROJECT_OVERVIEW.html",
                "PROJECT_OVERVIEW.html"
            ]
        )
        .is_some()
    );
    assert!(
        find_in_bundle(
            bundle_dir,
            &[
                "reports/insights/ONBOARDING_GUIDE.md",
                "ONBOARDING_GUIDE.md"
            ]
        )
        .is_some()
    );
    assert!(
        find_in_bundle(
            bundle_dir,
            &[
                "reports/insights/REVIEW_CHECKLIST.md",
                "REVIEW_CHECKLIST.md"
            ]
        )
        .is_some()
    );
}
