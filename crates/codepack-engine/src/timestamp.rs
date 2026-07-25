//! Timestamp helpers for this crate's report headers and bundle names.
//!
//! The calendar arithmetic itself lives in [`codepack_core::time`] — this module is only
//! the local naming layer over it, kept so the pipeline's call sites read in pipeline
//! terms ("the stamp that goes in a bundle name") rather than in calendar terms. Every
//! function here is a one-liner; there is no second implementation of anything.
//!
//! All values render **UTC**, not legacy's local wall clock — a deliberate,
//! long-standing deviation documented in [`codepack_core::time`]'s module doc: these
//! fields are cosmetic and nothing parses them back.

use std::time::SystemTime;

use codepack_core::time::{UtcDateTime, unix_seconds_of};

/// Compact `YYYYMMDD_HHMMSS` stamp for the export-bundle directory name, ported shape
/// from legacy `now_stamp()`. Used for uniqueness and human orientation only.
pub(crate) fn compact_utc_stamp() -> String {
    codepack_core::time::now_compact()
}

/// `YYYY-MM-DD HH:MM:SS` for "now" — the `Generated:` line of every report header this
/// crate writes, and `manifest.json`/`INDEX.md`'s `generated_at`. Analogous to legacy's
/// `human_now()`.
pub(crate) fn human_now_utc() -> String {
    codepack_core::time::now_human()
}

/// [`human_now_utc`] for an already-known [`SystemTime`] (a file's mtime, say) rather
/// than "now", so every timestamp field in a report renders identically.
pub(crate) fn human_from_system_time(time: SystemTime) -> String {
    UtcDateTime::from_unix_seconds(unix_seconds_of(time)).format_human()
}

/// Whole-second Unix epoch "now", for `codepack_storage`'s `NewExportRun.started_at`/
/// `finished_at`/`NewSnapshot.created_at` fields.
pub(crate) fn unix_timestamp_now() -> i64 {
    codepack_core::time::unix_timestamp_now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn compact_stamp_has_the_shape_bundle_names_depend_on() {
        let stamp = compact_utc_stamp();
        assert_eq!(stamp.len(), "19700101_000000".len());
        assert_eq!(stamp.chars().nth(8), Some('_'));
        assert!(
            stamp
                .chars()
                .filter(|c| *c != '_')
                .all(|c| c.is_ascii_digit())
        );
    }

    #[test]
    fn human_now_has_the_shape_report_headers_depend_on() {
        let stamp = human_now_utc();
        assert_eq!(stamp.len(), "2024-01-01 12:34:56".len());
        assert_eq!(stamp.chars().nth(4), Some('-'));
        assert_eq!(stamp.chars().nth(10), Some(' '));
    }

    #[test]
    fn human_from_system_time_formats_a_known_instant() {
        let instant = UNIX_EPOCH + Duration::from_secs(1_704_112_496);
        assert_eq!(human_from_system_time(instant), "2024-01-01 12:34:56");
    }

    #[test]
    fn human_from_system_time_clamps_pre_epoch_mtimes() {
        // A file whose mtime predates the epoch (corrupt metadata, or a filesystem that
        // reports 0 as "unknown") must not panic the export — it renders as the epoch.
        let before = UNIX_EPOCH - Duration::from_secs(10);
        assert_eq!(human_from_system_time(before), "1970-01-01 00:00:00");
    }

    #[test]
    fn unix_timestamp_now_is_a_plausible_recent_epoch_second() {
        assert!(unix_timestamp_now() > 1_700_000_000);
    }
}
