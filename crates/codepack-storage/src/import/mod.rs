//! Explicit, opt-in import of legacy `.project_exporter_history.json` data into
//! SQLite (BLUEPRINT §D.3, "импорт из старых JSON истории при первом запуске новой
//! версии"). Never called automatically on project open — a future caller (S9) wires
//! this into a one-time, user-visible migration step.
//!
//! Per-entry problems (a missing required field, an unparseable timestamp, a shape
//! mismatch) are collected into [`ImportReport::warnings`] and counted in `skipped`;
//! they never abort the rest of the array. Only a structural problem with the file
//! itself — unreadable, not valid JSON, or a top-level value that isn't a JSON array —
//! is a hard [`crate::StorageError`], matching legacy's own historical shape: a
//! top-level array of flat entry objects, nothing more exotic.
//!
//! A legacy entry with `cancelled: false` and an empty `snapshot: {}` (a real legacy
//! bug — a run that failed with copy errors but was never explicitly cancelled gets
//! this exact shape; see `docs/__arch__` `exporter.py`/`diff_service.py`) is imported
//! **faithfully**: a `snapshot` row with zero `snapshot_file` rows, not silently
//! dropped and not "fixed" into a `None`. The new write path
//! ([`crate::run::record_export_run`]) structurally prevents this shape for *new*
//! runs going forward — an `Option<Snapshot>` that is `None` can never be confused
//! with an empty-but-present snapshot the way a JSON `{}` was — but faithfully
//! reproducing already-recorded history is this module's entire job; it does not get
//! to silently rewrite the past.

mod entry;

use std::path::Path;

use rusqlite::Connection;

use crate::error::{Result, StorageError};
use entry::{ImportOutcome, LegacyHistoryEntry, import_entry};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

pub fn import_legacy_history(
    history_json_path: &Path,
    conn: &mut Connection,
) -> Result<ImportReport> {
    let raw = std::fs::read_to_string(history_json_path).map_err(|source| StorageError::Read {
        path: history_json_path.to_path_buf(),
        source,
    })?;

    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|source| StorageError::InvalidLegacyHistoryJson {
            path: history_json_path.to_path_buf(),
            source,
        })?;

    let Some(array) = value.as_array() else {
        return Err(StorageError::LegacyHistoryNotAnArray {
            path: history_json_path.to_path_buf(),
        });
    };

    let mut report = ImportReport::default();
    for (index, item) in array.iter().enumerate() {
        let entry: LegacyHistoryEntry = match serde_json::from_value(item.clone()) {
            Ok(entry) => entry,
            Err(_) => {
                report.skipped += 1;
                report.warnings.push(format!(
                    "entry {index}: does not match the expected legacy history entry shape"
                ));
                continue;
            }
        };

        match import_entry(conn, entry)? {
            ImportOutcome::Imported => report.imported += 1,
            ImportOutcome::Skipped(reason) => {
                report.skipped += 1;
                report.warnings.push(format!("entry {index}: {reason}"));
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::latest_snapshot;
    use crate::migrations::open;
    use crate::project::find_or_create_project;

    fn write_fixture(dir: &std::path::Path, contents: &str) -> std::path::PathBuf {
        let path = dir.join("history.json");
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn imports_successful_cancelled_poisoned_and_skips_malformed_entries() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();

        let fixture = write_fixture(
            dir.path(),
            r#"[
                {
                    "generated_at": "2024-01-02 03:04:05",
                    "project_name": "Demo",
                    "source_root": "C:\\repo\\demo",
                    "profile": "balanced",
                    "safe_export_mode": "safe",
                    "diff_export_mode": "all",
                    "incremental_export_enabled": false,
                    "result": "C:\\out\\demo.zip",
                    "split_archives": false,
                    "archives": ["C:\\out\\demo.zip"],
                    "copy_stats": {"files_copied": 12, "files_skipped_by_safety": 1, "errors": 0},
                    "tokens": 4096,
                    "snapshot": {
                        "src/main.rs": {"sha256": "abc123", "size": 100, "loc": 10}
                    },
                    "snapshot_stats": {"files": 1, "bytes": 100, "size_human": "100 B"},
                    "cancelled": false
                },
                {
                    "generated_at": "2024-01-03 03:04:05",
                    "project_name": "Demo",
                    "source_root": "C:\\repo\\demo",
                    "profile": "balanced",
                    "safe_export_mode": "safe",
                    "diff_export_mode": "all",
                    "result": "",
                    "archives": [],
                    "copy_stats": {"files_copied": 0, "files_skipped_by_safety": 0, "errors": 0},
                    "tokens": 0,
                    "snapshot": {},
                    "snapshot_stats": {"files": 0, "bytes": 0, "size_human": "0 B"},
                    "cancelled": true
                },
                {
                    "generated_at": "2024-01-04 03:04:05",
                    "project_name": "Demo",
                    "source_root": "C:\\repo\\demo",
                    "profile": "balanced",
                    "safe_export_mode": "safe",
                    "diff_export_mode": "all",
                    "result": "C:\\out\\demo.zip",
                    "archives": ["C:\\out\\demo.zip"],
                    "copy_stats": {"files_copied": 5, "files_skipped_by_safety": 0, "errors": 2},
                    "tokens": 512,
                    "snapshot": {},
                    "snapshot_stats": {"files": 0, "bytes": 0, "size_human": "0 B"},
                    "cancelled": false
                },
                {
                    "generated_at": "2024-01-05 03:04:05",
                    "project_name": "Demo",
                    "profile": "balanced",
                    "cancelled": false
                }
            ]"#,
        );

        let report = import_legacy_history(&fixture, &mut conn).unwrap();
        assert_eq!(report.imported, 3);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("source_root"));

        let project_id = find_or_create_project(&conn, "C:\\repo\\demo", "Demo", None).unwrap();

        let run_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM export_run WHERE project_id = ?1",
                rusqlite::params![project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(run_count, 3);

        let snapshot_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM snapshot WHERE project_id = ?1",
                rusqlite::params![project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            snapshot_count, 3,
            "every imported run had a `snapshot` key present, even if empty"
        );

        // The imported successful entry has one real snapshot file.
        let (_, files) = latest_snapshot(&conn, project_id).unwrap().unwrap();
        // The newest entry by generated_at is the "poisoned" one (2024-01-04), which
        // is faithfully imported as an empty snapshot, not fixed or dropped.
        assert!(
            files.is_empty(),
            "the most recent imported entry is the poisoned empty snapshot"
        );

        let poisoned_run_cancelled: bool = conn
            .query_row(
                "SELECT cancelled FROM export_run WHERE started_at = (SELECT MAX(started_at) FROM export_run WHERE project_id = ?1)",
                rusqlite::params![project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            !poisoned_run_cancelled,
            "the poisoned entry's cancelled flag is imported as-is: false"
        );

        let real_snapshot_file_count: i64 = conn
            .query_row(
                "SELECT file_count FROM snapshot WHERE created_at = (SELECT MIN(created_at) FROM snapshot WHERE project_id = ?1)",
                rusqlite::params![project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(real_snapshot_file_count, 1);
    }

    #[test]
    fn missing_file_is_a_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();
        let missing = dir.path().join("does-not-exist.json");
        assert!(import_legacy_history(&missing, &mut conn).is_err());
    }

    #[test]
    fn invalid_json_is_a_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();
        let fixture = write_fixture(dir.path(), "not json at all {{{");
        let err = import_legacy_history(&fixture, &mut conn).unwrap_err();
        assert!(matches!(err, StorageError::InvalidLegacyHistoryJson { .. }));
    }

    #[test]
    fn non_array_top_level_is_a_hard_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = open(&dir.path().join("codepack.db")).unwrap();
        let fixture = write_fixture(dir.path(), r#"{"not": "an array"}"#);
        let err = import_legacy_history(&fixture, &mut conn).unwrap_err();
        assert!(matches!(err, StorageError::LegacyHistoryNotAnArray { .. }));
    }
}
