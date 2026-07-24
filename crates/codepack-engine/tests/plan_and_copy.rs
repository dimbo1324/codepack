//! Integration coverage for steps 1 ("plan") and 2 ("copy") run back to back, the
//! same sequence a future orchestrator will run them in.

use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_engine::{build_export_paths, copy_project, run_export_plan};

#[test]
fn all_mode_plans_and_copies_every_file() {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join("src")).unwrap();
    fs::write(source.path().join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(source.path().join("README.md"), "hello").unwrap();

    let output = tempfile::tempdir().unwrap();
    let paths = build_export_paths(source.path(), output.path());
    let config = Config::default();
    let cancel = CancellationToken::new();

    let outcome = run_export_plan(&paths, &config, &HashMap::new(), None, &cancel).unwrap();
    assert!(outcome.include_relative_paths.is_none());

    let stats = copy_project(
        &outcome.export_plan,
        outcome.include_relative_paths.as_ref(),
        config.normalized_safe_export_mode(),
        &paths.source_root,
        &paths.project_dir,
        &cancel,
        &|_| {},
    )
    .unwrap();

    assert_eq!(stats.files_copied, 2);
    assert_eq!(stats.errors, 0);
    assert!(paths.project_dir.join("src/main.rs").is_file());
    assert!(paths.project_dir.join("README.md").is_file());
}

#[test]
fn safe_mode_excludes_an_env_file_and_counts_it_as_a_safety_skip() {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("main.py"), "print(1)").unwrap();
    fs::write(source.path().join(".env"), "SECRET=1").unwrap();

    let output = tempfile::tempdir().unwrap();
    let paths = build_export_paths(source.path(), output.path());
    let config = Config {
        safe_export_mode: "safe".to_string(),
        ..Config::default()
    };
    let cancel = CancellationToken::new();

    let outcome = run_export_plan(&paths, &config, &HashMap::new(), None, &cancel).unwrap();
    let stats = copy_project(
        &outcome.export_plan,
        outcome.include_relative_paths.as_ref(),
        config.normalized_safe_export_mode(),
        &paths.source_root,
        &paths.project_dir,
        &cancel,
        &|_| {},
    )
    .unwrap();

    assert_eq!(stats.files_copied, 1);
    assert_eq!(stats.files_skipped, 1);
    assert_eq!(stats.files_skipped_by_safety, 1);
    assert!(paths.project_dir.join("main.py").is_file());
    assert!(!paths.project_dir.join(".env").exists());
}

#[test]
fn file_override_rescues_an_exportignore_excluded_file_all_the_way_through_to_the_copy() {
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join(".exportignore"), "*.log\n").unwrap();
    fs::write(source.path().join("app.log"), "log contents").unwrap();
    fs::write(source.path().join("main.py"), "print(1)").unwrap();

    let output = tempfile::tempdir().unwrap();
    let paths = build_export_paths(source.path(), output.path());
    let config = Config::default();
    let cancel = CancellationToken::new();

    let mut overrides = HashMap::new();
    overrides.insert("app.log".to_string(), true);

    let outcome = run_export_plan(&paths, &config, &overrides, None, &cancel).unwrap();
    let stats = copy_project(
        &outcome.export_plan,
        outcome.include_relative_paths.as_ref(),
        config.normalized_safe_export_mode(),
        &paths.source_root,
        &paths.project_dir,
        &cancel,
        &|_| {},
    )
    .unwrap();

    // `.exportignore` itself is not excluded by its own rules, so it is a third
    // included/copied file alongside `app.log` (rescued) and `main.py`.
    assert_eq!(stats.files_copied, 3);
    assert!(paths.project_dir.join("app.log").is_file());
    assert!(paths.project_dir.join("main.py").is_file());
}

#[test]
fn cancellation_mid_copy_yields_an_honestly_partial_result_not_a_fabricated_complete_one() {
    let source = tempfile::tempdir().unwrap();
    for index in 0..20 {
        fs::write(source.path().join(format!("file{index:02}.txt")), "x").unwrap();
    }

    let output = tempfile::tempdir().unwrap();
    let paths = build_export_paths(source.path(), output.path());
    let config = Config::default();
    let cancel = CancellationToken::new();

    let outcome = run_export_plan(&paths, &config, &HashMap::new(), None, &cancel).unwrap();
    assert_eq!(outcome.export_plan.included_files.len(), 20);

    let cancel_for_log = cancel.clone();
    let processed = Mutex::new(0u32);
    let log = move |_: &str| {
        let mut count = processed.lock().unwrap();
        *count += 1;
        if *count == 7 {
            cancel_for_log.cancel();
        }
    };

    let stats = copy_project(
        &outcome.export_plan,
        outcome.include_relative_paths.as_ref(),
        config.normalized_safe_export_mode(),
        &paths.source_root,
        &paths.project_dir,
        &cancel,
        &log,
    )
    .unwrap();

    assert_eq!(stats.errors, 0);
    assert!(
        stats.files_copied >= 7,
        "files_copied = {}",
        stats.files_copied
    );
    assert!(
        stats.files_copied < 20,
        "files_copied = {} (expected an honestly partial result, not a fabricated complete one)",
        stats.files_copied
    );
}
