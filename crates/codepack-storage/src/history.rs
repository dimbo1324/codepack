//! [`list_export_runs`]: the read side of the export history.
//!
//! `codepack-storage` shipped in S5 with a complete write path and only one read
//! (`latest_snapshot`, which the `last_export` diff mode needs). Nothing could show a
//! user what had been exported, because nothing yet had a user — the CLI's `history`
//! command (S10) is the first caller, and this is the query it needs.
//!
//! Read-only by construction: no statement here writes, so nothing in this module can
//! disturb a snapshot baseline (invariant I6).

use std::path::Path;

use rusqlite::{Connection, params};

use crate::error::Result;

/// One row of export history, joined with the project it belongs to so a caller
/// listing runs across projects does not have to query names separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportRunRecord {
    pub id: i64,
    pub project_id: i64,
    pub project_name: String,
    pub project_root: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub profile: Option<String>,
    pub safe_mode: Option<String>,
    pub diff_mode: Option<String>,
    pub files_copied: Option<i64>,
    pub bytes_total: Option<i64>,
    pub tokens_est: Option<i64>,
    pub redacted_count: Option<i64>,
    pub cancelled: bool,
    pub result_path: Option<String>,
    /// Whether this run recorded a snapshot baseline. That is the storage layer's own
    /// definition of a successful run: `record_export_run` inserts a `snapshot` row
    /// only when the caller passed one, and it only does that when the export
    /// succeeded. Deriving success from `cancelled` alone would be wrong — a run that
    /// failed on copy errors is not cancelled.
    pub produced_snapshot: bool,
}

/// Newest first. `project_id` of `None` lists across every project.
///
/// `limit` is required rather than optional: history is unbounded by design (retention
/// is opt-in through `Config::history_keep_last_n`), so a caller that forgot to bound
/// the query would eventually load every run ever made into memory.
pub fn list_export_runs(
    conn: &Connection,
    project_id: Option<i64>,
    limit: usize,
) -> Result<Vec<ExportRunRecord>> {
    let sql = "SELECT r.id, r.project_id, p.name, p.root_path, r.started_at, r.finished_at,
                      r.profile, r.safe_mode, r.diff_mode, r.files_copied, r.bytes_total,
                      r.tokens_est, r.redacted_count, r.cancelled, r.result_path,
                      EXISTS(SELECT 1 FROM snapshot s WHERE s.run_id = r.id)
               FROM export_run r
               JOIN project p ON p.id = r.project_id
               WHERE (?1 IS NULL OR r.project_id = ?1)
               ORDER BY r.started_at DESC, r.id DESC
               LIMIT ?2";

    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map(params![project_id, limit as i64], |row| {
        Ok(ExportRunRecord {
            id: row.get(0)?,
            project_id: row.get(1)?,
            project_name: row.get(2)?,
            project_root: row.get(3)?,
            started_at: row.get(4)?,
            finished_at: row.get(5)?,
            profile: row.get(6)?,
            safe_mode: row.get(7)?,
            diff_mode: row.get(8)?,
            files_copied: row.get(9)?,
            bytes_total: row.get(10)?,
            tokens_est: row.get(11)?,
            redacted_count: row.get(12)?,
            cancelled: row.get(13)?,
            result_path: row.get(14)?,
            produced_snapshot: row.get(15)?,
        })
    })?;

    let mut records = Vec::new();
    for record in rows {
        records.push(record?);
    }
    Ok(records)
}

/// True when some export run's recorded `result_path` canonicalizes to `target`.
///
/// The question a caller validating a webview-supplied path actually asks — "did some
/// run of this installation produce exactly this file or directory" — needs neither the
/// full row nor the whole table: a `result_path` is only ever comparable after
/// canonicalizing it (`.` segments, a different case on Windows, a short 8.3 spelling all
/// resolve to the same target), and canonicalizing is a filesystem call SQL cannot do, so
/// candidates are narrowed here by file name before any of them touch the filesystem.
///
/// A `LIKE` pattern's `%`/`_` are wildcards, not literals, so a target file name that
/// happens to contain one only ever widens the candidate set — never narrows past a real
/// match — which is why the pattern below is not escaped: escaping could only turn a
/// true positive into a false negative, and an unescaped wildcard can only cost an extra,
/// harmless `canonicalize()` call that the equality check right after it then rejects.
///
/// This replaces a caller that used to fetch history with `list_export_runs(conn, None,
/// 0)` and search the result: `LIMIT 0` is SQLite for "zero rows", so that call always
/// returned an empty list and the check rejected every path unconditionally — including
/// ones this installation had just produced (audit 2026-09-07, S-2). Bounding by name in
/// SQL rather than "fetch everything, `usize::MAX` as the limit" also means a history of
/// many thousands of runs costs one indexed-enough `LIKE` scan and a handful of
/// `canonicalize()` calls, not a full table load followed by canonicalizing every row.
pub fn export_run_result_path_matches(conn: &Connection, target: &Path) -> Result<bool> {
    let Some(file_name) = target.file_name().and_then(|name| name.to_str()) else {
        return Ok(false);
    };
    let pattern = format!("%{file_name}");

    let mut statement =
        conn.prepare("SELECT result_path FROM export_run WHERE result_path LIKE ?1")?;
    let mut rows = statement.query(params![pattern])?;
    while let Some(row) = rows.next()? {
        let recorded: Option<String> = row.get(0)?;
        if let Some(recorded) = recorded
            && let Ok(canonical) = Path::new(&recorded).canonicalize()
            && canonical == target
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::open;
    use crate::project::find_or_create_project;
    use crate::run::record_export_run;
    use crate::types::{NewExportRun, NewSnapshot, NewSnapshotFile};

    /// The directory is returned, not leaked: dropping it is what deletes the database
    /// file when the test ends.
    fn temp_db() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = open(&dir.path().join("codepack.db")).unwrap();
        (dir, conn)
    }

    fn run(project_id: i64, started_at: i64) -> NewExportRun {
        NewExportRun {
            project_id,
            started_at,
            finished_at: Some(started_at + 1),
            profile: Some("full".to_string()),
            safe_mode: Some("balanced".to_string()),
            diff_mode: Some("all".to_string()),
            files_copied: Some(3),
            bytes_total: Some(100),
            tokens_est: Some(29),
            redacted_count: None,
            cancelled: false,
            result_path: Some("bundle.zip".to_string()),
        }
    }

    #[test]
    fn lists_newest_first_and_respects_the_limit() {
        let (_dir, mut conn) = temp_db();
        let project = find_or_create_project(&conn, "/tmp/a", "a", None).unwrap();
        for started_at in [10, 20, 30] {
            record_export_run(&mut conn, run(project, started_at), &[], &[], &[], None).unwrap();
        }

        let all = list_export_runs(&conn, None, 10).unwrap();
        assert_eq!(
            all.iter().map(|r| r.started_at).collect::<Vec<_>>(),
            vec![30, 20, 10]
        );

        let limited = list_export_runs(&conn, None, 2).unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].started_at, 30);
    }

    #[test]
    fn filters_by_project_and_carries_the_project_name() {
        let (_dir, mut conn) = temp_db();
        let a = find_or_create_project(&conn, "/tmp/a", "alpha", None).unwrap();
        let b = find_or_create_project(&conn, "/tmp/b", "beta", None).unwrap();
        record_export_run(&mut conn, run(a, 10), &[], &[], &[], None).unwrap();
        record_export_run(&mut conn, run(b, 20), &[], &[], &[], None).unwrap();

        let only_a = list_export_runs(&conn, Some(a), 10).unwrap();
        assert_eq!(only_a.len(), 1);
        assert_eq!(only_a[0].project_name, "alpha");
        assert_eq!(only_a[0].project_root, "/tmp/a");

        assert_eq!(list_export_runs(&conn, None, 10).unwrap().len(), 2);
    }

    #[test]
    fn produced_snapshot_distinguishes_a_successful_run_from_a_failed_one() {
        let (_dir, mut conn) = temp_db();
        let project = find_or_create_project(&conn, "/tmp/a", "a", None).unwrap();

        // A run that failed on copy errors: not cancelled, but no baseline recorded.
        record_export_run(&mut conn, run(project, 10), &[], &[], &[], None).unwrap();

        let snapshot = NewSnapshot {
            created_at: 20,
            file_count: 1,
            bytes_total: 10,
        };
        let files = vec![NewSnapshotFile {
            rel_path: "a.txt".to_string(),
            sha256: "x".to_string(),
            size_bytes: 10,
            loc: 1,
            mtime_ns: None,
        }];
        record_export_run(
            &mut conn,
            run(project, 20),
            &[],
            &[],
            &[],
            Some((snapshot, files.as_slice())),
        )
        .unwrap();

        let listed = list_export_runs(&conn, None, 10).unwrap();
        assert!(
            listed[0].produced_snapshot,
            "the run that recorded a baseline must read as successful"
        );
        assert!(
            !listed[1].produced_snapshot,
            "a run with no baseline must not read as successful just because it was \
             never cancelled"
        );
    }

    #[test]
    fn an_empty_database_lists_nothing_rather_than_erroring() {
        let (_dir, conn) = temp_db();
        assert!(list_export_runs(&conn, None, 10).unwrap().is_empty());
    }

    // --- export_run_result_path_matches (audit 2026-09-07, S-2/T-1) -------------------
    //
    // The acceptance branch that had no test at all before this pass: every existing
    // test on the path-validation feature exercised rejection, and a function that always
    // returns "no" passes every one of them. These are the tests that would have caught
    // `LIMIT 0` immediately.

    fn write_result_path(dir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, b"a real export artifact").unwrap();
        path.canonicalize().unwrap()
    }

    /// The exact case the defect broke: a run this installation actually produced.
    #[test]
    fn a_path_a_recorded_run_produced_is_matched() {
        let (dir, mut conn) = temp_db();
        let project = find_or_create_project(&conn, "/tmp/a", "a", None).unwrap();
        let bundle = write_result_path(&dir, "bundle.zip");

        let mut recorded = run(project, 10);
        recorded.result_path = Some(bundle.display().to_string());
        record_export_run(&mut conn, recorded, &[], &[], &[], None).unwrap();

        assert!(export_run_result_path_matches(&conn, &bundle).unwrap());
    }

    /// The paired rejection case: a path no run ever recorded is not matched, even when
    /// a run with a similarly named result exists in the same history.
    #[test]
    fn a_path_no_run_produced_is_not_matched() {
        let (dir, mut conn) = temp_db();
        let project = find_or_create_project(&conn, "/tmp/a", "a", None).unwrap();
        let recorded_bundle = write_result_path(&dir, "bundle.zip");
        let mut recorded = run(project, 10);
        recorded.result_path = Some(recorded_bundle.display().to_string());
        record_export_run(&mut conn, recorded, &[], &[], &[], None).unwrap();

        let stranger = dir.path().join("stranger.zip");
        std::fs::write(&stranger, b"not a real export").unwrap();
        let stranger = stranger.canonicalize().unwrap();

        assert!(!export_run_result_path_matches(&conn, &stranger).unwrap());
    }

    /// An empty history is a clean "no", not an error — the state before any export has
    /// ever run.
    #[test]
    fn an_empty_history_matches_nothing() {
        let (dir, conn) = temp_db();
        let anything = write_result_path(&dir, "whatever.zip");
        assert!(!export_run_result_path_matches(&conn, &anything).unwrap());
    }

    /// A run with no `result_path` at all (cancelled before archiving) must not be a
    /// false match for anything — the `LIKE` pattern excludes `NULL` on its own, and this
    /// is the test that would notice if a future rewrite stopped doing that.
    #[test]
    fn a_run_with_no_result_path_matches_nothing() {
        let (dir, mut conn) = temp_db();
        let project = find_or_create_project(&conn, "/tmp/a", "a", None).unwrap();
        let mut recorded = run(project, 10);
        recorded.result_path = None;
        record_export_run(&mut conn, recorded, &[], &[], &[], None).unwrap();

        let anything = write_result_path(&dir, "whatever.zip");
        assert!(!export_run_result_path_matches(&conn, &anything).unwrap());
    }

    /// Case and `.`-segment differences are exactly what canonicalization exists to
    /// absorb — the same claim the function's own doc comment makes about `.`, case on
    /// Windows, and 8.3 names all resolving to the same target.
    #[test]
    fn a_differently_spelled_path_to_the_same_file_still_matches() {
        let (dir, mut conn) = temp_db();
        let project = find_or_create_project(&conn, "/tmp/a", "a", None).unwrap();
        let bundle = write_result_path(&dir, "bundle.zip");

        let mut recorded = run(project, 10);
        recorded.result_path = Some(bundle.display().to_string());
        record_export_run(&mut conn, recorded, &[], &[], &[], None).unwrap();

        let respelled = dir.path().join(".").join("bundle.zip");
        assert!(export_run_result_path_matches(&conn, &respelled.canonicalize().unwrap()).unwrap());
    }
}
