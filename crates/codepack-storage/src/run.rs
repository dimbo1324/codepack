//! [`record_export_run`]: the single write path for an export run and everything it
//! produced. One transaction, one `export_run` row always inserted, a `snapshot`
//! (+ batched `snapshot_file` rows) inserted **iff** the caller passes `Some`.
//!
//! Invariant I6 (`docs/architecture/invariants.md`): nowhere in this crate does an
//! `UPDATE` ever touch `snapshot`/`snapshot_file`. A cancelled or failed run simply
//! passes `None` for `snapshot`, leaving whatever baseline was recorded by an earlier
//! successful run completely untouched — there is no code path that could overwrite
//! or partially corrupt it.

use rusqlite::{Connection, params};

use crate::error::Result;
use crate::types::{
    NewArchivePart, NewExportRun, NewFinding, NewRunFile, NewSnapshot, NewSnapshotFile,
};

#[allow(clippy::too_many_arguments)]
pub fn record_export_run(
    conn: &mut Connection,
    run: NewExportRun,
    files: &[NewRunFile],
    findings: &[NewFinding],
    archive_parts: &[NewArchivePart],
    snapshot: Option<(NewSnapshot, &[NewSnapshotFile])>,
) -> Result<i64> {
    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO export_run
            (project_id, started_at, finished_at, profile, safe_mode, diff_mode,
             files_copied, bytes_total, tokens_est, redacted_count, cancelled, result_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            run.project_id,
            run.started_at,
            run.finished_at,
            run.profile,
            run.safe_mode,
            run.diff_mode,
            run.files_copied,
            run.bytes_total,
            run.tokens_est,
            run.redacted_count,
            run.cancelled,
            run.result_path,
        ],
    )?;
    let run_id = tx.last_insert_rowid();

    {
        let mut stmt = tx.prepare(
            "INSERT INTO run_file (run_id, rel_path, size_bytes, loc, status, group_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for file in files {
            stmt.execute(params![
                run_id,
                file.rel_path,
                file.size_bytes,
                file.loc,
                file.status,
                file.group_name,
            ])?;
        }
    }

    {
        let mut stmt = tx.prepare(
            "INSERT INTO finding (run_id, type, severity, confidence, rule_id, file, line, message_redacted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for finding in findings {
            stmt.execute(params![
                run_id,
                finding.kind,
                finding.severity,
                finding.confidence,
                finding.rule_id,
                finding.file,
                finding.line,
                finding.message_redacted,
            ])?;
        }
    }

    {
        let mut stmt = tx.prepare(
            "INSERT INTO archive_part (run_id, part_index, archive_name, compressed_bytes, groups)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for part in archive_parts {
            stmt.execute(params![
                run_id,
                part.part_index,
                part.archive_name,
                part.compressed_bytes,
                part.groups,
            ])?;
        }
    }

    if let Some((new_snapshot, snapshot_files)) = snapshot {
        tx.execute(
            "INSERT INTO snapshot (project_id, run_id, created_at, file_count, bytes_total)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                run.project_id,
                run_id,
                new_snapshot.created_at,
                new_snapshot.file_count,
                new_snapshot.bytes_total,
            ],
        )?;
        let snapshot_id = tx.last_insert_rowid();

        let mut stmt = tx.prepare(
            "INSERT INTO snapshot_file (snapshot_id, rel_path, sha256, size_bytes, loc, mtime_ns)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for file in snapshot_files {
            stmt.execute(params![
                snapshot_id,
                file.rel_path,
                file.sha256,
                file.size_bytes,
                file.loc,
                file.mtime_ns,
            ])?;
        }

        // `project.last_export_at` tracks the last *successful* export, matching
        // BLUEPRINT SS A.9's history model (a cancelled/failed run is recorded in
        // `export_run` for history purposes but never counts as "the last export" a
        // user-facing "last exported: <date>" indicator should show) — deliberately
        // tied to the same `Some(snapshot)` gate that decides whether this run
        // produced a baseline at all, not to every attempt regardless of outcome.
        tx.execute(
            "UPDATE project SET last_export_at = ?1 WHERE id = ?2",
            params![run.started_at, run.project_id],
        )?;
    }

    tx.commit()?;
    Ok(run_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::latest_snapshot;
    use crate::migrations::open;
    use crate::project::find_or_create_project;

    fn base_run(project_id: i64, started_at: i64, cancelled: bool) -> NewExportRun {
        NewExportRun {
            project_id,
            started_at,
            finished_at: Some(started_at + 5),
            profile: Some("balanced".to_string()),
            safe_mode: Some("safe".to_string()),
            diff_mode: Some("all".to_string()),
            files_copied: Some(3),
            bytes_total: Some(1024),
            tokens_est: Some(256),
            redacted_count: Some(0),
            cancelled,
            result_path: Some("/out/export.zip".to_string()),
        }
    }

    #[test]
    fn always_inserts_export_run_regardless_of_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();
        let project_id = find_or_create_project(&conn, "/repo", "Repo", None).unwrap();

        let run_id = record_export_run(
            &mut conn,
            base_run(project_id, 100, false),
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();
        assert!(run_id > 0);

        let run_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM export_run", [], |row| row.get(0))
            .unwrap();
        assert_eq!(run_count, 1);
    }

    #[test]
    fn last_export_at_only_advances_on_a_run_that_produced_a_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();
        let project_id = find_or_create_project(&conn, "/repo", "Repo", None).unwrap();

        // A run with no snapshot (cancelled/failed) must not advance last_export_at.
        record_export_run(
            &mut conn,
            base_run(project_id, 100, true),
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();
        let last_export_at: Option<i64> = conn
            .query_row(
                "SELECT last_export_at FROM project WHERE id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(last_export_at, None);

        // A run that produces a snapshot (successful) does advance it.
        let snapshot_files = vec![NewSnapshotFile {
            rel_path: "a.txt".to_string(),
            sha256: "aaaa".to_string(),
            size_bytes: 1,
            loc: 1,
            mtime_ns: None,
        }];
        let new_snapshot = NewSnapshot {
            created_at: 200,
            file_count: 1,
            bytes_total: 1,
        };
        record_export_run(
            &mut conn,
            base_run(project_id, 200, false),
            &[],
            &[],
            &[],
            Some((new_snapshot, &snapshot_files)),
        )
        .unwrap();
        let last_export_at: i64 = conn
            .query_row(
                "SELECT last_export_at FROM project WHERE id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(last_export_at, 200);
    }

    #[test]
    fn a_run_with_no_snapshot_leaves_a_previously_recorded_baseline_completely_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();
        let project_id = find_or_create_project(&conn, "/repo", "Repo", None).unwrap();

        let snapshot_files = vec![NewSnapshotFile {
            rel_path: "src/main.rs".to_string(),
            sha256: "deadbeef".to_string(),
            size_bytes: 42,
            loc: 7,
            mtime_ns: Some(123_456_789),
        }];
        let new_snapshot = NewSnapshot {
            created_at: 100,
            file_count: 1,
            bytes_total: 42,
        };
        record_export_run(
            &mut conn,
            base_run(project_id, 100, false),
            &[],
            &[],
            &[],
            Some((new_snapshot, &snapshot_files)),
        )
        .unwrap();

        let (baseline_before, files_before) = latest_snapshot(&conn, project_id).unwrap().unwrap();

        // Simulate a later run that fails/gets cancelled: the caller passes `None`.
        record_export_run(
            &mut conn,
            base_run(project_id, 200, true),
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();

        let (baseline_after, files_after) = latest_snapshot(&conn, project_id).unwrap().unwrap();
        assert_eq!(baseline_before, baseline_after);
        assert_eq!(files_before, files_after);
    }
}
