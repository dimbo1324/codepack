//! [`cleanup_old_runs`]: retention of the last N `export_run` rows per project.
//!
//! `keep_last_n` is a plain caller-supplied parameter, not a crate-level default
//! constant: legacy's `MAX_HISTORY_ITEMS = 50` is a single global cap shared across
//! every project's history combined (`export_history.py`), which BLUEPRINT §D.3
//! overstates as "configurable retention of last N runs *per project*". The new
//! per-project relational retention this function implements is a deliberate,
//! logged improvement over the legacy global cap, not a literal port of it — see
//! the S5 task report for the owner decision this should be recorded against.
//!
//! Deletion is a single `DELETE FROM export_run` targeting only the rows to drop;
//! every child table (`run_file`, `finding`, `archive_part`, `snapshot` and, via a
//! second cascade, `snapshot_file`) is cleaned up entirely through `ON DELETE CASCADE`
//! — this module never issues a second, manual `DELETE` against any child table.

use rusqlite::{Connection, params};

use crate::error::Result;

pub fn cleanup_old_runs(conn: &Connection, project_id: i64, keep_last_n: usize) -> Result<usize> {
    let keep_last_n = i64::try_from(keep_last_n).unwrap_or(i64::MAX);
    let deleted = conn.execute(
        "DELETE FROM export_run
         WHERE project_id = ?1
           AND id NOT IN (
               SELECT id FROM export_run
               WHERE project_id = ?1
               ORDER BY started_at DESC, id DESC
               LIMIT ?2
           )",
        params![project_id, keep_last_n],
    )?;
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::open;
    use crate::project::find_or_create_project;
    use crate::run::record_export_run;
    use crate::types::{
        NewArchivePart, NewExportRun, NewFinding, NewRunFile, NewSnapshot, NewSnapshotFile,
    };

    fn run(project_id: i64, started_at: i64) -> NewExportRun {
        NewExportRun {
            project_id,
            started_at,
            finished_at: Some(started_at),
            profile: None,
            safe_mode: None,
            diff_mode: None,
            files_copied: None,
            bytes_total: None,
            tokens_est: None,
            redacted_count: None,
            cancelled: false,
            result_path: None,
        }
    }

    fn table_count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    #[test]
    fn keeps_exactly_the_n_most_recent_runs_and_cascades_every_child_table() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();
        let project_id = find_or_create_project(&conn, "/repo", "Repo", None).unwrap();

        let total_runs = 7;
        let keep = 3;
        for index in 0..total_runs {
            let started_at = 100 + index;
            let files = vec![NewRunFile {
                rel_path: format!("file_{index}.txt"),
                size_bytes: Some(10),
                loc: Some(1),
                status: Some("included".to_string()),
                group_name: None,
            }];
            let findings = vec![NewFinding {
                kind: "potential_secret".to_string(),
                severity: "high".to_string(),
                confidence: "high".to_string(),
                rule_id: "secret_like_line".to_string(),
                file: format!("file_{index}.txt"),
                line: Some(1),
                message_redacted: "<REDACTED>".to_string(),
            }];
            let archive_parts = vec![NewArchivePart {
                part_index: 0,
                archive_name: format!("part_{index}.zip"),
                compressed_bytes: Some(1024),
                groups: None,
            }];
            let snapshot_files = vec![NewSnapshotFile {
                rel_path: format!("file_{index}.txt"),
                sha256: "abc".to_string(),
                size_bytes: 10,
                loc: 1,
                mtime_ns: None,
            }];
            record_export_run(
                &mut conn,
                run(project_id, started_at),
                &files,
                &findings,
                &archive_parts,
                Some((
                    NewSnapshot {
                        created_at: started_at,
                        file_count: 1,
                        bytes_total: 10,
                    },
                    &snapshot_files,
                )),
            )
            .unwrap();
        }

        assert_eq!(table_count(&conn, "export_run"), total_runs);
        assert_eq!(table_count(&conn, "run_file"), total_runs);
        assert_eq!(table_count(&conn, "finding"), total_runs);
        assert_eq!(table_count(&conn, "archive_part"), total_runs);
        assert_eq!(table_count(&conn, "snapshot"), total_runs);
        assert_eq!(table_count(&conn, "snapshot_file"), total_runs);

        let deleted = cleanup_old_runs(&conn, project_id, keep as usize).unwrap();
        assert_eq!(deleted, (total_runs - keep) as usize);

        assert_eq!(table_count(&conn, "export_run"), keep);
        assert_eq!(table_count(&conn, "run_file"), keep);
        assert_eq!(table_count(&conn, "finding"), keep);
        assert_eq!(table_count(&conn, "archive_part"), keep);
        assert_eq!(table_count(&conn, "snapshot"), keep);
        assert_eq!(table_count(&conn, "snapshot_file"), keep);

        let mut stmt = conn
            .prepare("SELECT started_at FROM export_run ORDER BY started_at DESC")
            .unwrap();
        let remaining: Vec<i64> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|value| value.unwrap())
            .collect();
        assert_eq!(remaining, vec![106, 105, 104]);
    }

    #[test]
    fn keeping_more_than_exist_deletes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();
        let project_id = find_or_create_project(&conn, "/repo", "Repo", None).unwrap();
        record_export_run(&mut conn, run(project_id, 100), &[], &[], &[], None).unwrap();

        let deleted = cleanup_old_runs(&conn, project_id, 50).unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(table_count(&conn, "export_run"), 1);
    }
}
