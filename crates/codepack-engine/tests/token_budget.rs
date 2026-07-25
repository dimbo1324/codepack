//! "Fit to budget" (BLUEPRINT §B.3) actually reaches the pipeline.
//!
//! `codepack_tokens::fit_to_budget` shipped in S6 with no caller, so ROADMAP §6 claimed
//! the capability while no export ever invoked it. These tests pin that it is wired,
//! that it is off by default, and that it prioritizes by importance rather than by
//! whatever order the walk happened to produce.

mod support;

use std::collections::HashMap;
use std::fs;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_storage::open;

fn build_fixture(root: &std::path::Path) {
    // An entrypoint the key-files ranking scores highly, plus bulky filler it does not.
    fs::write(root.join("main.py"), "print('entrypoint')\n").unwrap();
    fs::write(root.join("package.json"), "{\"name\":\"demo\"}\n").unwrap();
    for index in 0..6 {
        fs::write(root.join(format!("filler_{index}.txt")), "x".repeat(4_000)).unwrap();
    }
}

/// Returns the outcome together with the temporary directories, so the caller keeps
/// them alive while inspecting what was written.
fn export(
    config: &Config,
) -> (
    codepack_engine::ExportOutcome,
    tempfile::TempDir,
    tempfile::TempDir,
) {
    let source = tempfile::tempdir().unwrap();
    build_fixture(source.path());
    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut conn = open(&db_dir.path().join("codepack.db")).unwrap();
    let (tx, _rx) = codepack_core::progress_channel();

    let outcome = codepack_engine::run_export(
        &mut conn,
        source.path(),
        output.path(),
        config,
        &HashMap::new(),
        &tx,
        &CancellationToken::new(),
    )
    .unwrap();
    (outcome, source, output)
}

#[test]
fn no_budget_is_the_default_and_keeps_every_file() {
    let (outcome, _source, _output) = export(&Config::default());
    let plan = &outcome.analytics.as_ref().unwrap().replan;
    assert!(
        plan.included_files.len() >= 8,
        "default config must not drop anything: {:?}",
        plan.included_files.len()
    );
}

#[test]
fn a_small_budget_drops_files_and_keeps_the_ranked_entrypoint() {
    // Roughly two filler files' worth of tokens: enough to force a real choice.
    let config = Config {
        token_budget: 2_400,
        keep_staging_folder: true,
        ..Config::default()
    };
    let (outcome, _source, _output) = export(&config);

    let copied: Vec<String> = fs::read_dir(&outcome.paths.project_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();

    assert!(
        copied.len() < 8,
        "a budget this small must exclude something, got {copied:?}"
    );
    assert!(
        copied.iter().any(|name| name == "main.py"),
        "the entrypoint outranks bulk filler and must survive the budget, got {copied:?}"
    );
}

#[test]
fn a_pinned_file_survives_a_budget_that_would_otherwise_drop_it() {
    // `always_include_files` is an explicit user instruction; the budget is a heuristic.
    // Without this the pass silently discarded pinned files on importance/tokens alone.
    let config = Config {
        token_budget: 100,
        keep_staging_folder: true,
        always_include_files: vec!["filler_3.txt".to_string()],
        ..Config::default()
    };
    let (outcome, _source, _output) = export(&config);

    let copied: Vec<String> = fs::read_dir(&outcome.paths.project_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();

    assert!(
        copied.iter().any(|name| name == "filler_3.txt"),
        "a pinned file must outrank the budget, got {copied:?}"
    );
}
