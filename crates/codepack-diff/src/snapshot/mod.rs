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
