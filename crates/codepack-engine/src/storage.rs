//! Hand-written conversions between `codepack_diff::Snapshot`/`SnapshotFile` and
//! `codepack_storage::NewSnapshot`/`NewSnapshotFile`/`StoredSnapshotFile`.
//!
//! Neither crate provides a `From` impl across this boundary on purpose:
//! `codepack-storage`'s own module doc states it depends on no other `codepack-*`
//! crate and defines its own local `New*`/`Stored*` types precisely so a future caller
//! (this crate) populates them by copying fields, never via a `From` impl living in
//! either crate (which would create a forbidden dependency in one direction or the
//! other). This crate sits above both `codepack-diff` and `codepack-storage`, so it is
//! the correct — and only — place either direction can live.

use codepack_diff::{Snapshot, SnapshotFile};
use codepack_storage::{NewSnapshot, NewSnapshotFile, StoredSnapshotFile};

/// The read-side direction: a previously persisted baseline becomes a real
/// [`Snapshot`] so [`crate::plan::run_export_plan`]'s `previous_snapshot` parameter can
/// feed it straight into `last_export` diff-mode resolution.
pub(crate) fn stored_snapshot_to_diff_snapshot(files: Vec<StoredSnapshotFile>) -> Snapshot {
    Snapshot {
        files: files
            .into_iter()
            .map(|file| {
                let rel_path = file.rel_path;
                (
                    rel_path.clone(),
                    SnapshotFile {
                        rel_path,
                        sha256: file.sha256,
                        size: u64::try_from(file.size_bytes).unwrap_or(0),
                        loc: u64::try_from(file.loc).unwrap_or(0),
                        mtime_ns: file.mtime_ns.unwrap_or(0),
                    },
                )
            })
            .collect(),
    }
}

/// The write-side direction: a freshly computed [`Snapshot`] becomes the pair
/// [`record_export_run`](codepack_storage::record_export_run) needs to persist a new
/// baseline for a successful run.
pub(crate) fn diff_snapshot_to_new_snapshot(
    snapshot: &Snapshot,
    created_at: i64,
) -> (NewSnapshot, Vec<NewSnapshotFile>) {
    let files: Vec<NewSnapshotFile> = snapshot
        .files
        .values()
        .map(|file| NewSnapshotFile {
            rel_path: file.rel_path.clone(),
            sha256: file.sha256.clone(),
            size_bytes: i64::try_from(file.size).unwrap_or(i64::MAX),
            loc: i64::try_from(file.loc).unwrap_or(i64::MAX),
            mtime_ns: Some(file.mtime_ns),
        })
        .collect();

    let new_snapshot = NewSnapshot {
        created_at,
        file_count: i64::try_from(snapshot.len()).unwrap_or(i64::MAX),
        bytes_total: i64::try_from(snapshot.total_bytes()).unwrap_or(i64::MAX),
    };

    (new_snapshot, files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_stored_files() -> Vec<StoredSnapshotFile> {
        vec![
            StoredSnapshotFile {
                id: 1,
                snapshot_id: 10,
                rel_path: "src\\main.rs".to_string(),
                sha256: "deadbeef".to_string(),
                size_bytes: 42,
                loc: 7,
                mtime_ns: Some(123_456_789),
            },
            StoredSnapshotFile {
                id: 2,
                snapshot_id: 10,
                rel_path: "README.md".to_string(),
                sha256: "cafef00d".to_string(),
                size_bytes: 5,
                loc: 1,
                mtime_ns: None,
            },
        ]
    }

    #[test]
    fn stored_snapshot_round_trips_every_file_field_except_the_row_ids() {
        let stored = sample_stored_files();
        let snapshot = stored_snapshot_to_diff_snapshot(stored);

        assert_eq!(snapshot.len(), 2);
        let main_rs = &snapshot.files["src\\main.rs"];
        assert_eq!(main_rs.sha256, "deadbeef");
        assert_eq!(main_rs.size, 42);
        assert_eq!(main_rs.loc, 7);
        assert_eq!(main_rs.mtime_ns, 123_456_789);

        let readme = &snapshot.files["README.md"];
        assert_eq!(readme.mtime_ns, 0, "a missing mtime_ns falls back to 0");
    }

    #[test]
    fn diff_snapshot_to_new_snapshot_round_trips_every_file_field() {
        let snapshot = Snapshot {
            files: [(
                "src\\lib.rs".to_string(),
                SnapshotFile {
                    rel_path: "src\\lib.rs".to_string(),
                    sha256: "abc123".to_string(),
                    size: 100,
                    loc: 12,
                    mtime_ns: 999,
                },
            )]
            .into_iter()
            .collect(),
        };

        let (new_snapshot, files) = diff_snapshot_to_new_snapshot(&snapshot, 555);

        assert_eq!(new_snapshot.created_at, 555);
        assert_eq!(new_snapshot.file_count, 1);
        assert_eq!(new_snapshot.bytes_total, 100);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].rel_path, "src\\lib.rs");
        assert_eq!(files[0].sha256, "abc123");
        assert_eq!(files[0].size_bytes, 100);
        assert_eq!(files[0].loc, 12);
        assert_eq!(files[0].mtime_ns, Some(999));
    }

    #[test]
    fn a_full_round_trip_through_both_directions_preserves_every_file() {
        let stored = sample_stored_files();
        let snapshot = stored_snapshot_to_diff_snapshot(stored);
        let (new_snapshot, new_files) = diff_snapshot_to_new_snapshot(&snapshot, 42);

        assert_eq!(new_snapshot.file_count, 2);
        assert_eq!(new_snapshot.bytes_total, 47);

        let main_rs = new_files
            .iter()
            .find(|file| file.rel_path == "src\\main.rs")
            .expect("main.rs survives the round trip");
        assert_eq!(main_rs.sha256, "deadbeef");
        assert_eq!(main_rs.size_bytes, 42);
        assert_eq!(main_rs.loc, 7);
        assert_eq!(main_rs.mtime_ns, Some(123_456_789));
    }
}
