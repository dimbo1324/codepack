//! [`Snapshot`]/[`SnapshotFile`]: per-file `sha256`/`size`/`loc`/`mtime_ns`, mapping
//! 1:1 onto the `SNAPSHOT`/`SNAPSHOT_FILE` schema (BLUEPRINT §D.2) even though this
//! stage does not persist anything itself — persistence is `codepack-storage`'s job,
//! stage S5.

mod hash;
mod loc;
mod walk;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub use hash::{HASH_CHUNK_BYTES, hash_file};
pub use loc::count_loc_if_countable;
pub use walk::snapshot_project;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub rel_path: String,
    pub sha256: String,
    pub size: u64,
    pub loc: u64,
    pub mtime_ns: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub files: HashMap<String, SnapshotFile>,
}

impl Snapshot {
    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn total_bytes(&self) -> u64 {
        self.files.values().map(|file| file.size).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot_of(entries: &[(&str, u64)]) -> Snapshot {
        Snapshot {
            files: entries
                .iter()
                .map(|(rel_path, size)| {
                    (
                        (*rel_path).to_string(),
                        SnapshotFile {
                            rel_path: (*rel_path).to_string(),
                            sha256: "0".repeat(64),
                            size: *size,
                            loc: 1,
                            mtime_ns: 0,
                        },
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn an_empty_snapshot_reports_no_files_and_no_bytes() {
        let snapshot = Snapshot::default();
        assert!(snapshot.is_empty());
        assert_eq!(snapshot.len(), 0);
        assert_eq!(snapshot.total_bytes(), 0);
    }

    #[test]
    fn totals_sum_every_file() {
        // These two values become `NewSnapshot.file_count`/`bytes_total` in the history
        // row, so they are what a later `last_export` comparison is measured against.
        let snapshot = snapshot_of(&[("a.rs", 10), ("b.rs", 32), ("c.rs", 0)]);
        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.len(), 3);
        assert_eq!(snapshot.total_bytes(), 42);
    }

    #[test]
    fn a_snapshot_round_trips_through_json() {
        // The snapshot is persisted and read back across runs, so its serialized shape
        // is a compatibility surface (BLUEPRINT §D.2).
        let snapshot = snapshot_of(&[("src\\main.rs", 7)]);
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: Snapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, snapshot);
    }

    #[test]
    fn serialized_field_names_match_the_documented_schema() {
        let snapshot = snapshot_of(&[("a.rs", 1)]);
        let json = serde_json::to_string(&snapshot).unwrap();
        for field in ["rel_path", "sha256", "size", "loc", "mtime_ns"] {
            assert!(json.contains(field), "missing field {field} in {json}");
        }
    }
}
