//! `NewFinding`/`NewSnapshotFile` round-trip. Field names and values are copied
//! directly from reading `crates/codepack-security/src/scan/mod.rs`'s `Finding`
//! struct and `crates/codepack-diff/src/snapshot/mod.rs`'s `SnapshotFile` struct
//! (this crate does not depend on either), so the fixtures below are shaped exactly
//! like their real output.

use codepack_storage::{
    NewExportRun, NewFinding, NewSnapshot, NewSnapshotFile, find_or_create_project, open,
    record_export_run,
};

#[test]
fn finding_and_snapshot_file_round_trip_field_for_field() {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = open(&dir.path().join("codepack.db")).unwrap();
    let project_id = find_or_create_project(&conn, "/repo", "Repo", None).unwrap();

    // Shaped like `codepack_security::scan::Finding { kind: FindingKind::PotentialSecret,
    // severity: "high", confidence: "high", file: "app.py", line: Some(2),
    // rule: "aws-access-key-id", message: "<REDACTED>" }`.
    let finding = NewFinding {
        kind: "potential_secret".to_string(),
        severity: "high".to_string(),
        confidence: "high".to_string(),
        rule_id: "aws-access-key-id".to_string(),
        file: "app.py".to_string(),
        line: Some(2),
        message_redacted: "<REDACTED>".to_string(),
    };

    // Shaped like `codepack_diff::snapshot::SnapshotFile { rel_path: "src/main.rs",
    // sha256: <64 hex chars>, size: 1234u64, loc: 42u64, mtime_ns: 1_700_000_000_000i64 }`.
    let snapshot_file = NewSnapshotFile {
        rel_path: "src/main.rs".to_string(),
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b85".to_string(),
        size_bytes: 1234,
        loc: 42,
        mtime_ns: Some(1_700_000_000_000),
    };

    let run_id = record_export_run(
        &mut conn,
        NewExportRun {
            project_id,
            started_at: 1_000,
            finished_at: Some(1_005),
            profile: Some("balanced".to_string()),
            safe_mode: Some("safe".to_string()),
            diff_mode: Some("all".to_string()),
            files_copied: Some(1),
            bytes_total: Some(1234),
            tokens_est: Some(300),
            redacted_count: Some(1),
            cancelled: false,
            result_path: Some("/out/export.zip".to_string()),
        },
        &[],
        std::slice::from_ref(&finding),
        &[],
        Some((
            NewSnapshot {
                created_at: 1_000,
                file_count: 1,
                bytes_total: 1234,
            },
            std::slice::from_ref(&snapshot_file),
        )),
    )
    .unwrap();

    let stored_finding: (String, String, String, String, String, Option<i64>, String) = conn
        .query_row(
            "SELECT type, severity, confidence, rule_id, file, line, message_redacted
             FROM finding WHERE run_id = ?1",
            rusqlite::params![run_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .unwrap();

    assert_eq!(stored_finding.0, finding.kind);
    assert_eq!(stored_finding.1, finding.severity);
    assert_eq!(stored_finding.2, finding.confidence);
    assert_eq!(stored_finding.3, finding.rule_id);
    assert_eq!(stored_finding.4, finding.file);
    assert_eq!(stored_finding.5, finding.line);
    assert_eq!(stored_finding.6, finding.message_redacted);

    let snapshot_id: i64 = conn
        .query_row(
            "SELECT id FROM snapshot WHERE run_id = ?1",
            rusqlite::params![run_id],
            |row| row.get(0),
        )
        .unwrap();

    let stored_snapshot_file: (String, String, i64, i64, Option<i64>) = conn
        .query_row(
            "SELECT rel_path, sha256, size_bytes, loc, mtime_ns
             FROM snapshot_file WHERE snapshot_id = ?1",
            rusqlite::params![snapshot_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();

    assert_eq!(stored_snapshot_file.0, snapshot_file.rel_path);
    assert_eq!(stored_snapshot_file.1, snapshot_file.sha256);
    assert_eq!(stored_snapshot_file.2, snapshot_file.size_bytes);
    assert_eq!(stored_snapshot_file.3, snapshot_file.loc);
    assert_eq!(stored_snapshot_file.4, snapshot_file.mtime_ns);
}
