//! Locally-defined `New*` write-side structs and `Stored*` read-side row structs.
//!
//! Deliberately **not** `From<security::Finding>`/`From<diff::SnapshotFile>` etc.: this
//! crate never depends on `codepack-security`/`codepack-diff` (scope boundary, S5).
//! A future caller (stage S9's engine) populates these by copying fields out of those
//! crates' real output types.

/// Maps onto `export_run` (all columns but `id`, which SQLite assigns).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewExportRun {
    pub project_id: i64,
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
}

/// Maps onto `run_file` (all columns but `id`/`run_id`, which are assigned by
/// [`crate::run::record_export_run`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRunFile {
    pub rel_path: String,
    pub size_bytes: Option<i64>,
    pub loc: Option<i64>,
    pub status: Option<String>,
    pub group_name: Option<String>,
}

/// Maps onto `finding`. Field names follow the column names, not
/// `codepack_security::scan::Finding`'s Rust names verbatim: `kind` -> `type`,
/// `rule` -> `rule_id`, `message` -> `message_redacted` (already redacted by the
/// caller — invariant I3; this crate performs zero redaction of its own).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFinding {
    pub kind: String,
    pub severity: String,
    pub confidence: String,
    pub rule_id: String,
    pub file: String,
    pub line: Option<i64>,
    pub message_redacted: String,
}

/// Maps onto `archive_part` (all columns but `id`/`run_id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewArchivePart {
    pub part_index: i64,
    pub archive_name: String,
    pub compressed_bytes: Option<i64>,
    pub groups: Option<String>,
}

/// Maps onto `snapshot` (all columns but `id`/`project_id`/`run_id`, which
/// [`crate::run::record_export_run`] fills in from its own arguments).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSnapshot {
    pub created_at: i64,
    pub file_count: i64,
    pub bytes_total: i64,
}

/// Maps onto `snapshot_file` (all columns but `id`/`snapshot_id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSnapshotFile {
    pub rel_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub loc: i64,
    pub mtime_ns: Option<i64>,
}

/// Read-side row returned by [`crate::baseline::latest_snapshot`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSnapshot {
    pub id: i64,
    pub project_id: i64,
    pub run_id: Option<i64>,
    pub created_at: i64,
    pub file_count: i64,
    pub bytes_total: i64,
}

/// Read-side row returned by [`crate::baseline::latest_snapshot`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSnapshotFile {
    pub id: i64,
    pub snapshot_id: i64,
    pub rel_path: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub loc: i64,
    pub mtime_ns: Option<i64>,
}

/// Whole-second Unix epoch timestamp for "now", used when a caller does not supply
/// one explicitly (`find_or_create_project`'s `created_at`, migration bookkeeping).
/// Falls back to `0` rather than panicking if the system clock is somehow set before
/// the Unix epoch — a `created_at` of `0` is a harmless, self-evidently-wrong sentinel,
/// never a crash.
pub(crate) fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}
