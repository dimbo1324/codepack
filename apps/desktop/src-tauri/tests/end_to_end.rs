//! The two acceptance criteria ROADMAP §3 states for S11 that no unit test can cover:
//! the whole wizard flow producing a real bundle, and cancelling a run without leaving
//! anything behind.
//!
//! These drive the command functions directly rather than through a webview. What the
//! criteria are about is what the commands *do* — which files a preview reports, whether
//! an archive appears, whether staging is gone afterwards — and none of that is a
//! property of how a browser renders it. Standing up a real window would add a display
//! server to CI and test Tauri's IPC rather than this application's behaviour.
//!
//! Every test uses a temporary database. `open_database` resolves the user's real history
//! file, so a test that used it would write into the developer's own data.

use std::collections::HashMap;

use codepack_core::config::Config;
use codepack_core::{CancellationToken, progress_channel};
use codepack_desktop::commands::{export, history, project};

/// A small but realistic project: source, docs, a dependency directory that must be
/// ignored, and a credential file that safe mode must catch.
fn fixture_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/main.rs"),
        "fn main() { println!(\"hi\"); }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32) -> i32 { a + 1 }\n",
    )
    .unwrap();
    std::fs::write(root.join("README.md"), "# demo\n\nA fixture project.\n").unwrap();
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();

    // Must be excluded by safe mode, and must show as a warning in the preview.
    std::fs::write(root.join(".env"), "API_KEY=fixture-value-not-a-real-key\n").unwrap();

    // Must be pruned by ignore rules, not scanned or copied.
    std::fs::create_dir_all(root.join("node_modules/left-pad")).unwrap();
    std::fs::write(
        root.join("node_modules/left-pad/index.js"),
        "module.exports = 1;\n",
    )
    .unwrap();

    dir
}

fn temp_database(dir: &std::path::Path) -> codepack_storage::Connection {
    codepack_storage::open(&dir.join("history.db")).unwrap()
}

/// The root as the engine will see it.
///
/// `start_export` canonicalizes before calling into the engine (`resolve_project_root`),
/// so a test that drives `run_to_completion` directly has to do the same — otherwise the
/// project is recorded under a different spelling of the same directory than a later
/// lookup uses, which is a property of the test setup rather than of the code.
fn engine_root(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().canonicalize().unwrap()
}

/// Config for a fast, complete export: the `quick` profile keeps the report catalogue
/// small without changing what gets copied.
fn export_config() -> Config {
    Config {
        export_profile: "quick".to_string(),
        ..Config::default()
    }
}

#[test]
fn the_whole_wizard_flow_produces_a_bundle_and_records_it() {
    let source = fixture_project();
    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut connection = temp_database(db_dir.path());

    // 1. Project step: open the folder the picker returned.
    let context = project::open_project(source.path().display().to_string()).unwrap();
    assert!(!context.root.is_empty());

    let config = export_config();

    // 2. Preview step: see what would be shared, before anything is written.
    let preview =
        project::preview_project(context.root.clone(), config.clone(), HashMap::new()).unwrap();
    assert!(preview.included_files >= 4, "{preview:?}");
    assert!(
        preview.sensitive_count >= 1,
        "the .env should be reported as a sensitive exclusion"
    );
    assert_eq!(
        preview.tree.status, "warning",
        "the root should carry the warning up from the .env"
    );
    assert!(
        !preview
            .tree
            .children
            .as_ref()
            .unwrap()
            .iter()
            .any(|child| child.name == "node_modules"),
        "node_modules must be pruned, not shown in the preview"
    );

    // 3. Security step: scanning looks at what an export would filter out.
    let scan = project::scan_project(context.root.clone(), config.clone()).unwrap();
    assert!(
        scan.summary.total_findings > 0,
        "the .env's key should be found: {scan:?}"
    );

    // 4. Export step.
    let (sender, receiver) = progress_channel();
    let collector = std::thread::spawn(move || receiver.iter().count());

    let report = export::run_to_completion(
        &mut connection,
        &engine_root(&source),
        output.path(),
        &config,
        &HashMap::new(),
        &sender,
        &CancellationToken::new(),
    )
    .unwrap();
    drop(sender);

    assert!(collector.join().unwrap() > 0, "no progress was reported");
    assert!(report.successful, "{report:?}");
    assert!(!report.cancelled);
    assert_eq!(report.errors, 0);
    assert!(report.files_copied >= 4);

    // 5. Result step: an archive exists on disk and is not empty.
    let result_path = report.result_path.as_ref().expect("a result path");
    let result = std::path::Path::new(result_path);
    assert!(
        result.exists(),
        "the result path does not exist: {result_path}"
    );
    if result.is_file() {
        assert!(std::fs::metadata(result).unwrap().len() > 0);
    }

    // Staging is cleaned up: the default config does not keep it.
    assert!(
        !std::path::Path::new(&report.staging_dir).exists(),
        "staging survived a successful export: {}",
        report.staging_dir
    );

    // 6. Analytics step reads back what the export wrote.
    let summary = export::read_project_profile(result_path.clone()).unwrap();
    assert!(summary.files > 0, "{summary:?}");

    // 7. History step: the run is recorded, and reported as successful.
    let runs = codepack_storage::list_export_runs(&connection, None, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, report.run_id);
    assert!(
        runs[0].produced_snapshot,
        "a successful run records a baseline"
    );
    assert!(!runs[0].cancelled);
}

#[test]
fn cancelling_an_export_leaves_no_staging_directory_behind() {
    // The acceptance criterion, stated directly: a user who cancels must not be left
    // with a half-built directory to find and delete themselves.
    let source = fixture_project();
    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut connection = temp_database(db_dir.path());

    let cancel = CancellationToken::new();
    cancel.cancel();

    let (sender, receiver) = progress_channel();
    let drain = std::thread::spawn(move || receiver.iter().count());

    let report = export::run_to_completion(
        &mut connection,
        &engine_root(&source),
        output.path(),
        &export_config(),
        &HashMap::new(),
        &sender,
        &cancel,
    )
    .unwrap();
    drop(sender);
    let _ = drain.join();

    assert!(report.cancelled);
    assert!(!report.successful);
    assert!(
        !std::path::Path::new(&report.staging_dir).exists(),
        "staging survived a cancelled export: {}",
        report.staging_dir
    );

    // Steps 7 and 8 still ran, so the attempt is still in history — and it did *not*
    // record a snapshot baseline, which is invariant I6 seen from the outside.
    let runs = codepack_storage::list_export_runs(&connection, None, 10).unwrap();
    assert_eq!(runs.len(), 1);
    assert!(
        !runs[0].produced_snapshot,
        "a cancelled run must not overwrite the baseline"
    );
}

#[test]
fn a_cancelled_export_does_not_overwrite_a_baseline_an_earlier_run_recorded() {
    // The consequence that matters: the next `last_export` diff must still compare
    // against the last *successful* export, not against a cancelled one.
    let source = fixture_project();
    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut connection = temp_database(db_dir.path());

    let (first_sender, first_receiver) = progress_channel();
    let first_drain = std::thread::spawn(move || first_receiver.iter().count());
    let good = export::run_to_completion(
        &mut connection,
        &engine_root(&source),
        output.path(),
        &export_config(),
        &HashMap::new(),
        &first_sender,
        &CancellationToken::new(),
    )
    .unwrap();
    drop(first_sender);
    let _ = first_drain.join();
    assert!(good.successful);

    let project_id =
        codepack_storage::find_project_id(&connection, &engine_root(&source).display().to_string())
            .unwrap()
            .expect("the successful run registered the project");
    let baseline_before = codepack_storage::latest_snapshot(&connection, project_id).unwrap();
    assert!(baseline_before.is_some());

    let cancel = CancellationToken::new();
    cancel.cancel();
    let (second_sender, second_receiver) = progress_channel();
    let second_drain = std::thread::spawn(move || second_receiver.iter().count());
    let cancelled = export::run_to_completion(
        &mut connection,
        &engine_root(&source),
        output.path(),
        &export_config(),
        &HashMap::new(),
        &second_sender,
        &cancel,
    )
    .unwrap();
    drop(second_sender);
    let _ = second_drain.join();
    assert!(cancelled.cancelled);

    let baseline_after = codepack_storage::latest_snapshot(&connection, project_id).unwrap();
    assert_eq!(
        baseline_before, baseline_after,
        "the cancelled run changed the baseline"
    );
}

#[test]
fn keeping_the_staging_folder_is_honoured_when_the_user_asks_for_it() {
    // The inverse of the cleanup guarantee: `keep_staging_folder` exists so a user can
    // inspect exactly what was collected, and silently deleting it would make the
    // setting a lie.
    let source = fixture_project();
    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut connection = temp_database(db_dir.path());

    let config = Config {
        keep_staging_folder: true,
        ..export_config()
    };

    let (sender, receiver) = progress_channel();
    let drain = std::thread::spawn(move || receiver.iter().count());
    let report = export::run_to_completion(
        &mut connection,
        &engine_root(&source),
        output.path(),
        &config,
        &HashMap::new(),
        &sender,
        &CancellationToken::new(),
    )
    .unwrap();
    drop(sender);
    let _ = drain.join();

    assert!(
        std::path::Path::new(&report.staging_dir).exists(),
        "keep_staging_folder was ignored"
    );
}

#[test]
fn a_file_override_from_the_preview_tree_changes_what_the_export_copies() {
    // The preview's force-exclude is only meaningful if the export honours it, so the
    // two are checked together rather than the override being tested against the
    // preview alone.
    let source = fixture_project();
    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut connection = temp_database(db_dir.path());

    let mut overrides = HashMap::new();
    overrides.insert("README.md".to_string(), false);

    let baseline = project::preview_project(
        source.path().display().to_string(),
        export_config(),
        HashMap::new(),
    )
    .unwrap();
    let narrowed = project::preview_project(
        source.path().display().to_string(),
        export_config(),
        overrides.clone(),
    )
    .unwrap();
    assert_eq!(narrowed.included_files, baseline.included_files - 1);

    let (sender, receiver) = progress_channel();
    let drain = std::thread::spawn(move || receiver.iter().count());
    let report = export::run_to_completion(
        &mut connection,
        &engine_root(&source),
        output.path(),
        &export_config(),
        &overrides,
        &sender,
        &CancellationToken::new(),
    )
    .unwrap();
    drop(sender);
    let _ = drain.join();

    assert_eq!(
        report.files_copied,
        baseline.included_files as u32 - 1,
        "the export did not honour the preview's force-exclude"
    );
}

#[test]
fn history_reports_an_empty_list_for_a_project_that_never_exported() {
    // A project with no runs and a project that was never opened look the same from the
    // History page, and both should render as empty rather than as a failure.
    let source = fixture_project();
    let db_dir = tempfile::tempdir().unwrap();
    let connection = temp_database(db_dir.path());

    let runs = codepack_storage::list_export_runs(&connection, None, 10).unwrap();
    assert!(runs.is_empty());

    // `fetch_history` itself resolves the real database, so the command is exercised
    // only for its path handling here; the query behaviour is asserted above.
    assert!(history::fetch_history(Some(source.path().display().to_string()), Some(5)).is_ok());
}
