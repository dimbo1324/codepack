//! Shared helpers for the S9 verification suite (`task-checklist.md`'s S9
//! Verification section): a small multi-file-type fixture builder, and a driver that
//! runs [`run_export`] while deterministically injecting cancellation around a chosen
//! pipeline-step boundary by watching the real progress channel — never a sleep-based
//! timing race.

#![allow(dead_code)]

use std::fs;
use std::io::Read as _;
use std::path::Path;

use codepack_core::CancellationToken;
use codepack_core::ProgressEvent;
use codepack_core::config::Config;
use codepack_engine::{ExportOutcome, run_export};
use codepack_storage::Connection;

pub(crate) fn init_git_repo(root: &Path) {
    let repo = git2::Repository::init(root).unwrap();
    fs::write(root.join("tracked.txt"), "hello from git\n").unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test", "test@example.local").unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "initial commit",
        &tree,
        &[],
    )
    .unwrap();
}

/// A fixture combining a few different file types, per this pass's shape-parity
/// criterion: a `.py` file, a `.md` file, a `.env` file (safety-mode coverage), and a
/// small one-commit git repo.
pub(crate) fn build_multi_type_fixture(source: &Path) {
    fs::write(source.join("main.py"), "print('hello')\n").unwrap();
    fs::write(source.join("README.md"), "# Fixture project\n").unwrap();
    fs::write(source.join(".env"), "API_KEY=super-secret-value\n").unwrap();
    init_git_repo(source);
}

pub(crate) fn read_zip_entry(zip_path: &Path, name: &str) -> String {
    let file = fs::File::open(zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut entry = archive.by_name(name).unwrap();
    let mut contents = String::new();
    entry.read_to_string(&mut contents).unwrap();
    contents
}

pub(crate) fn zip_entry_names(zip_path: &Path) -> Vec<String> {
    let file = fs::File::open(zip_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    (0..archive.len())
        .map(|index| archive.by_index(index).unwrap().name().to_string())
        .collect()
}

/// Where, relative to a pipeline step's own progress narration, this helper flips the
/// shared [`CancellationToken`]. See this file's own module doc comment and
/// `task-checklist.md`'s S9 Verification notes for why step 6 ("analytics") uses
/// [`CancelTrigger::OnStepStarted`] rather than [`CancelTrigger::AfterStepFinished`]:
/// step 6 is the pipeline's last real step before the final "have we been cancelled"
/// latch that feeds the recorded `export_run.cancelled` flag runs
/// (`orchestrator.rs`'s `cancelled = cancelled || cancel.is_cancelled();`), with no
/// further step's own real I/O time in between to absorb the watcher thread's wake-up
/// latency — cancelling on `StepStarted` instead gives the token time to become visible
/// during step 6's own (real, multi-report) work.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CancelTrigger {
    AfterStepFinished(&'static str),
    OnStepStarted(&'static str),
}

/// Runs [`run_export`] once, cancelling the token the moment the progress channel
/// reports the event `trigger` names — deterministic because it synchronizes on a real
/// event instead of a sleep, per this crate's own `copy.rs`/`structure.rs` precedent for
/// controlled (not timing-raced) cancellation tests.
pub(crate) fn run_export_with_trigger(
    conn: &mut Connection,
    source_root: &Path,
    output_root: &Path,
    config: &Config,
    trigger: CancelTrigger,
) -> ExportOutcome {
    let cancel = CancellationToken::new();
    let (tx, rx) = codepack_core::progress_channel();

    std::thread::scope(|scope| {
        let cancel_for_watcher = cancel.clone();
        scope.spawn(move || {
            for event in rx.iter() {
                let hit = match (&event, trigger) {
                    (
                        ProgressEvent::StepFinished { step },
                        CancelTrigger::AfterStepFinished(name),
                    ) => step == name,
                    (ProgressEvent::StepStarted { step }, CancelTrigger::OnStepStarted(name)) => {
                        step == name
                    }
                    _ => false,
                };
                if hit {
                    cancel_for_watcher.cancel();
                }
            }
        });

        let outcome = run_export(
            conn,
            source_root,
            output_root,
            config,
            &std::collections::HashMap::new(),
            &tx,
            &cancel,
        )
        .unwrap();

        drop(tx);
        outcome
    })
}

/// Runs a plain, uncancelled export against `source_root`/`conn` to seed a baseline
/// snapshot row, matching S5's own baseline-unchanged assertion pattern
/// (`codepack-storage`'s `run.rs`:
/// `a_run_with_no_snapshot_leaves_a_previously_recorded_baseline_completely_unchanged`).
pub(crate) fn seed_successful_baseline(
    conn: &mut Connection,
    source_root: &Path,
    output_root: &Path,
    config: &Config,
) -> ExportOutcome {
    let cancel = CancellationToken::new();
    let (tx, _rx) = codepack_core::progress_channel();
    run_export(
        conn,
        source_root,
        output_root,
        config,
        &std::collections::HashMap::new(),
        &tx,
        &cancel,
    )
    .unwrap()
}
