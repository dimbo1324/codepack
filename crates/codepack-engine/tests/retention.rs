//! History retention actually prunes (decision Q10, 2026-07-25).
//!
//! `codepack_storage::cleanup_old_runs` shipped in S5 but had no caller until the
//! core-hardening pass, so `export_run` rows grew without bound no matter what any
//! configuration said. These tests pin the wiring, not the pruning algorithm — the
//! algorithm itself is covered inside `codepack-storage`.

mod support;

use std::collections::HashMap;
use std::fs;

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_storage::{Connection, open};
use support::build_multi_type_fixture;

fn export_once(conn: &mut Connection, source: &std::path::Path, config: &Config) {
    let output = tempfile::tempdir().unwrap();
    let (tx, _rx) = codepack_core::progress_channel();
    codepack_engine::run_export(
        conn,
        source,
        output.path(),
        config,
        &HashMap::new(),
        &tx,
        &CancellationToken::new(),
    )
    .unwrap();
}

fn run_rows(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM export_run", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn keeps_only_the_configured_number_of_runs() {
    let source = tempfile::tempdir().unwrap();
    build_multi_type_fixture(source.path());
    let db_dir = tempfile::tempdir().unwrap();
    let mut conn = open(&db_dir.path().join("codepack.db")).unwrap();

    let config = Config {
        history_keep_last_n: 2,
        ..Config::default()
    };
    for _ in 0..4 {
        export_once(&mut conn, source.path(), &config);
    }

    assert_eq!(
        run_rows(&conn),
        2,
        "four exports with keep_last_n = 2 must leave two rows"
    );
}

#[test]
fn zero_disables_pruning_entirely() {
    let source = tempfile::tempdir().unwrap();
    build_multi_type_fixture(source.path());
    let db_dir = tempfile::tempdir().unwrap();
    let mut conn = open(&db_dir.path().join("codepack.db")).unwrap();

    let config = Config {
        history_keep_last_n: 0,
        ..Config::default()
    };
    for _ in 0..3 {
        export_once(&mut conn, source.path(), &config);
    }

    assert_eq!(run_rows(&conn), 3, "keep_last_n = 0 must keep everything");
}

#[test]
fn a_run_that_redacted_something_records_a_real_count_not_null() {
    let source = tempfile::tempdir().unwrap();
    build_multi_type_fixture(source.path());
    // The fixture's `.env` never reaches the bundle (safe mode excludes it), so the
    // count must come from a file that really is exported. `redacted_count` reports
    // what the text dump rewrote, not what the scanner detected.
    fs::write(
        source.path().join("settings.py"),
        "API_KEY = \"placeholder-value\"\nDEBUG = True\n",
    )
    .unwrap();

    let db_dir = tempfile::tempdir().unwrap();
    let mut conn = open(&db_dir.path().join("codepack.db")).unwrap();
    export_once(&mut conn, source.path(), &Config::default());

    let recorded: Option<i64> = conn
        .query_row("SELECT redacted_count FROM export_run", [], |row| {
            row.get(0)
        })
        .unwrap();

    assert_eq!(
        recorded,
        Some(1),
        "the text dump redacted one line, so history must record 1 rather than NULL"
    );
}
