//! [`latest_snapshot`]: the read side of the `last_export` diff mode's baseline
//! lookup. Ordered strictly by `created_at DESC` (ties broken by `id DESC`, the
//! insertion order) — no other filtering. Invariant I6 is enforced entirely on the
//! write side ([`crate::run::record_export_run`]): a cancelled/failed *new* run never
//! creates a `snapshot` row in the first place, so this simple "give me the newest
//! one" query can never surface a corrupted baseline for new data. Imported legacy
//! history rows are a deliberate exception — see `crate::import`'s module docs.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::Result;
use crate::types::{StoredSnapshot, StoredSnapshotFile};

pub fn latest_snapshot(
    conn: &Connection,
    project_id: i64,
) -> Result<Option<(StoredSnapshot, Vec<StoredSnapshotFile>)>> {
    let snapshot = conn
        .query_row(
            "SELECT id, project_id, run_id, created_at, file_count, bytes_total
             FROM snapshot
             WHERE project_id = ?1
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
            params![project_id],
            |row| {
                Ok(StoredSnapshot {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    run_id: row.get(2)?,
                    created_at: row.get(3)?,
                    file_count: row.get(4)?,
                    bytes_total: row.get(5)?,
                })
            },
        )
        .optional()?;

    let Some(snapshot) = snapshot else {
        return Ok(None);
    };

    let mut stmt = conn.prepare(
        "SELECT id, snapshot_id, rel_path, sha256, size_bytes, loc, mtime_ns
         FROM snapshot_file
         WHERE snapshot_id = ?1
         ORDER BY rel_path",
    )?;
    let files = stmt
        .query_map(params![snapshot.id], |row| {
            Ok(StoredSnapshotFile {
                id: row.get(0)?,
                snapshot_id: row.get(1)?,
                rel_path: row.get(2)?,
                sha256: row.get(3)?,
                size_bytes: row.get(4)?,
                loc: row.get(5)?,
                mtime_ns: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(Some((snapshot, files)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::open;
    use crate::project::find_or_create_project;
    use crate::run::record_export_run;
    use crate::types::{NewExportRun, NewSnapshot, NewSnapshotFile};

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

    #[test]
    fn no_snapshot_yet_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("codepack.db")).unwrap();
        let project_id = find_or_create_project(&conn, "/repo", "Repo", None).unwrap();
        assert!(latest_snapshot(&conn, project_id).unwrap().is_none());
    }

    #[test]
    fn returns_the_most_recently_created_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();
        let project_id = find_or_create_project(&conn, "/repo", "Repo", None).unwrap();

        let older_files = vec![NewSnapshotFile {
            rel_path: "a.txt".to_string(),
            sha256: "old".to_string(),
            size_bytes: 1,
            loc: 1,
            mtime_ns: None,
        }];
        record_export_run(
            &mut conn,
            run(project_id, 100),
            &[],
            &[],
            &[],
            Some((
                NewSnapshot {
                    created_at: 100,
                    file_count: 1,
                    bytes_total: 1,
                },
                &older_files,
            )),
        )
        .unwrap();

        let newer_files = vec![NewSnapshotFile {
            rel_path: "b.txt".to_string(),
            sha256: "new".to_string(),
            size_bytes: 2,
            loc: 2,
            mtime_ns: None,
        }];
        record_export_run(
            &mut conn,
            run(project_id, 200),
            &[],
            &[],
            &[],
            Some((
                NewSnapshot {
                    created_at: 200,
                    file_count: 1,
                    bytes_total: 2,
                },
                &newer_files,
            )),
        )
        .unwrap();

        let (snapshot, files) = latest_snapshot(&conn, project_id).unwrap().unwrap();
        assert_eq!(snapshot.created_at, 200);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, "b.txt");
    }
}
