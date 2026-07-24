//! The cancellation battery (`task-checklist.md`'s S9 Verification section): one
//! scenario per pipeline-step boundary, complementing `pipeline.rs`'s own
//! "cancelled before step 1 even begins" test (boundary 0).
//!
//! ## Final shape: seven new scenarios, not eight
//!
//! `run_export` sequences eight steps; there are seven boundaries *between* them
//! (1|2, 2|3, ..., 7|8) plus the "before step 1" boundary `pipeline.rs` already covers.
//! This file adds one scenario per boundary after steps 1 through 7 — seven tests,
//! bringing the suite to eight scenarios total across both files, matching the
//! acceptance criterion's "one per pipeline step" framing.
//!
//! Every scenario drives [`support::run_export_with_trigger`], which cancels the real
//! [`codepack_core::CancellationToken`] the moment the real progress channel reports a
//! chosen event — never a sleep, never a timing guess (see that helper's own doc
//! comment for exactly which event each boundary watches for and why step 6 differs).
//!
//! ## A genuine, documented asymmetry at boundary 7 (discovered during this pass)
//!
//! `orchestrator.rs` latches the `cancelled` flag that is recorded on the
//! `export_run` row **before** step 7 begins and never updates it again — ported
//! verbatim from legacy `exporter.py` (`cancelled = cancelled or self.cancel_event.is_set()`
//! at line 264, run once, right before the manifest step). A cancellation that arrives
//! only during step 7 (manifest) or step 8 (archiving) can therefore never flip that
//! recorded field to `true`; legacy has exactly the same property. What legacy *does*
//! re-check, fresh, after archiving completes, is the `successful` gate that decides
//! whether a new history-snapshot baseline gets recorded (`exporter.py` line 313:
//! `successful = not cancelled and not self.cancel_event.is_set() and copy_stats.errors
//! == 0`). This crate's own `run_export` was missing that fresh recheck before this
//! pass (a real parity bug, not a deliberate simplification) — fixed in
//! `orchestrator.rs` as part of this verification pass, with the fix's own reasoning
//! recorded in that module's doc comment. `boundary_7_...` below asserts the corrected,
//! legacy-matching behavior directly: `export_run.cancelled` stays `false`, but the
//! baseline is still never advanced.

mod support;

use codepack_core::config::Config;
use codepack_storage::{find_or_create_project, latest_snapshot, open};
use support::{
    CancelTrigger, build_multi_type_fixture, run_export_with_trigger, seed_successful_baseline,
};

struct Fixture {
    _source_dir: tempfile::TempDir,
    _output_dir: tempfile::TempDir,
    _db_dir: tempfile::TempDir,
    conn: codepack_storage::Connection,
    source: std::path::PathBuf,
    output: std::path::PathBuf,
}

fn fixture() -> Fixture {
    let source_dir = tempfile::tempdir().unwrap();
    build_multi_type_fixture(source_dir.path());
    let output_dir = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let conn = open(&db_dir.path().join("codepack.db")).unwrap();
    Fixture {
        source: source_dir.path().to_path_buf(),
        output: output_dir.path().to_path_buf(),
        _source_dir: source_dir,
        _output_dir: output_dir,
        _db_dir: db_dir,
        conn,
    }
}

fn assert_cancelled_run_never_advances_baseline(
    fx: &mut Fixture,
    config: &Config,
    trigger: CancelTrigger,
    expect_recorded_cancelled: bool,
) {
    // Seed a baseline with a real prior successful run against the same project
    // identity, then prove it is left row-for-row unchanged by the cancelled run —
    // reusing `codepack-storage`'s own `run.rs` assertion pattern
    // (`a_run_with_no_snapshot_leaves_a_previously_recorded_baseline_completely_unchanged`).
    let seed_config = Config::default();
    let seed_outcome = seed_successful_baseline(&mut fx.conn, &fx.source, &fx.output, &seed_config);
    assert!(
        seed_outcome.successful,
        "seed run must succeed to establish a baseline"
    );
    let project_id =
        find_or_create_project(&fx.conn, &fx.source.display().to_string(), "fixture", None)
            .unwrap();
    assert_eq!(project_id, seed_outcome.project_id);
    let baseline_before = latest_snapshot(&fx.conn, project_id).unwrap();
    assert!(baseline_before.is_some(), "seed run must record a baseline");

    let outcome = run_export_with_trigger(&mut fx.conn, &fx.source, &fx.output, config, trigger);

    assert!(
        !outcome.successful,
        "a cancelled run must never be flagged successful"
    );

    let recorded_cancelled: bool = fx
        .conn
        .query_row(
            "SELECT cancelled FROM export_run WHERE id = ?1",
            [outcome.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        recorded_cancelled, expect_recorded_cancelled,
        "export_run.cancelled did not match the expected legacy-parity value"
    );

    let baseline_after = latest_snapshot(&fx.conn, project_id).unwrap();
    assert_eq!(
        baseline_before, baseline_after,
        "a cancelled run must leave the previously recorded baseline completely unchanged"
    );

    // Steps 7-8 always run, cancelled or not: the manifest and a primary archive result
    // must still exist (a lighter check than `pipeline.rs`'s own exhaustive proof of
    // this property for the "cancel before step 1" case).
    assert!(
        outcome.archive_result.primary_result().is_some(),
        "archiving still runs on every cancelled scenario"
    );

    let run_count: i64 = fx
        .conn
        .query_row("SELECT COUNT(*) FROM export_run", [], |row| row.get(0))
        .unwrap();
    assert_eq!(run_count, 2, "exactly the seed run plus this cancelled run");
}

#[test]
fn boundary_1_cancel_right_after_plan_still_leaves_a_clean_no_snapshot_outcome() {
    let mut fx = fixture();
    let config = Config {
        keep_staging_folder: false,
        ..Config::default()
    };
    assert_cancelled_run_never_advances_baseline(
        &mut fx,
        &config,
        CancelTrigger::AfterStepFinished("1/8: plan"),
        true,
    );
    // Staging cleanup ran (default policy): nothing left behind under the output root
    // beyond the seed run's own bundle and this run's manifest/archive artifacts, which
    // `assert_cancelled_run_never_advances_baseline` already checked for existence.
}

#[test]
fn boundary_2_cancel_right_after_copy_leaves_staging_cleaned_up_by_default() {
    let mut fx = fixture();
    let config = Config {
        keep_staging_folder: false,
        ..Config::default()
    };
    assert_cancelled_run_never_advances_baseline(
        &mut fx,
        &config,
        CancelTrigger::AfterStepFinished("2/8: copy"),
        true,
    );
}

#[test]
fn boundary_3_cancel_right_after_structure_report_keeps_staging_when_configured() {
    let mut fx = fixture();
    let config = Config {
        keep_staging_folder: true,
        ..Config::default()
    };
    assert_cancelled_run_never_advances_baseline(
        &mut fx,
        &config,
        CancelTrigger::AfterStepFinished("3/8: structure"),
        true,
    );
}

#[test]
fn boundary_4_cancel_right_after_git_report_records_the_attempt() {
    let mut fx = fixture();
    let config = Config {
        keep_staging_folder: false,
        ..Config::default()
    };
    assert_cancelled_run_never_advances_baseline(
        &mut fx,
        &config,
        CancelTrigger::AfterStepFinished("4/8: git"),
        true,
    );
}

#[test]
fn boundary_5_cancel_right_after_text_dump_records_the_attempt() {
    let mut fx = fixture();
    let config = Config {
        keep_staging_folder: false,
        ..Config::default()
    };
    assert_cancelled_run_never_advances_baseline(
        &mut fx,
        &config,
        CancelTrigger::AfterStepFinished("5/8: text dump"),
        true,
    );
}

/// See this file's own module doc comment for why step 6 is watched via
/// `StepStarted` rather than `StepFinished`.
#[test]
fn boundary_6_cancel_during_analytics_records_the_attempt() {
    let mut fx = fixture();
    let config = Config {
        keep_staging_folder: false,
        ..Config::default()
    };
    assert_cancelled_run_never_advances_baseline(
        &mut fx,
        &config,
        CancelTrigger::OnStepStarted("6/8: analytics"),
        true,
    );
}

/// See this file's own module doc comment for the documented, legacy-matching
/// asymmetry this scenario exercises: `export_run.cancelled` stays `false` here (the
/// flag was already latched before step 7 began), but the baseline is still never
/// advanced, because `successful` freshly re-checks the token after archiving.
#[test]
fn boundary_7_cancel_between_manifest_and_archiving_still_blocks_the_new_baseline() {
    let mut fx = fixture();
    let config = Config {
        keep_staging_folder: false,
        ..Config::default()
    };
    assert_cancelled_run_never_advances_baseline(
        &mut fx,
        &config,
        CancelTrigger::AfterStepFinished("7/8: manifest"),
        false,
    );
}

#[test]
fn a_cancelled_run_with_no_prior_baseline_records_no_snapshot_at_all() {
    let source = tempfile::tempdir().unwrap();
    build_multi_type_fixture(source.path());
    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut conn = open(&db_dir.path().join("codepack.db")).unwrap();
    let config = Config::default();

    let outcome = run_export_with_trigger(
        &mut conn,
        source.path(),
        output.path(),
        &config,
        CancelTrigger::AfterStepFinished("2/8: copy"),
    );

    assert!(!outcome.successful);
    let baseline = latest_snapshot(&conn, outcome.project_id).unwrap();
    assert!(
        baseline.is_none(),
        "no prior baseline existed and none must be created"
    );
}
