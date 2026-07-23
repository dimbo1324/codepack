//! The legacy `.project_exporter_history.json` entry shape (`export_history.py`,
//! `exporter.py`'s `append_export_history` call site) and the mapping from one
//! deserialized entry to this crate's `New*` rows.

use std::collections::HashMap;

use rusqlite::Connection;
use serde::Deserialize;

use crate::error::Result;
use crate::project::find_or_create_project;
use crate::run::record_export_run;
use crate::types::{NewArchivePart, NewExportRun, NewSnapshot, NewSnapshotFile};

#[derive(Debug, Deserialize)]
pub(super) struct LegacyHistoryEntry {
    #[serde(default)]
    pub generated_at: Option<String>,
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub source_root: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub safe_export_mode: Option<String>,
    #[serde(default)]
    pub diff_export_mode: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub archives: Vec<String>,
    #[serde(default)]
    pub copy_stats: Option<LegacyCopyStats>,
    #[serde(default)]
    pub tokens: Option<i64>,
    #[serde(default)]
    pub snapshot: Option<HashMap<String, LegacySnapshotFileMeta>>,
    #[serde(default)]
    pub cancelled: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct LegacyCopyStats {
    #[serde(default)]
    pub files_copied: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LegacySnapshotFileMeta {
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub size: Option<i64>,
    #[serde(default)]
    pub loc: Option<i64>,
}

pub(super) enum ImportOutcome {
    Imported,
    Skipped(&'static str),
}

/// Validates and persists one already-deserialized legacy entry. Returns
/// `Ok(Skipped(reason))` — never an `Err` — for a structurally incomplete entry, so
/// [`super::import_legacy_history`] can keep processing the rest of the array. An
/// `Err` here means a genuine SQLite failure, not a data-quality problem.
pub(super) fn import_entry(
    conn: &mut Connection,
    entry: LegacyHistoryEntry,
) -> Result<ImportOutcome> {
    let Some(source_root) = non_empty(entry.source_root) else {
        return Ok(ImportOutcome::Skipped(
            "missing or empty required field 'source_root'",
        ));
    };
    let Some(project_name) = non_empty(entry.project_name) else {
        return Ok(ImportOutcome::Skipped(
            "missing or empty required field 'project_name'",
        ));
    };
    let Some(generated_at) = entry.generated_at else {
        return Ok(ImportOutcome::Skipped(
            "missing required field 'generated_at'",
        ));
    };
    let Some(timestamp) = parse_naive_datetime_to_unix(&generated_at) else {
        return Ok(ImportOutcome::Skipped(
            "field 'generated_at' is not a recognized timestamp format",
        ));
    };

    let project_id = find_or_create_project(conn, &source_root, &project_name, None)?;
    let cancelled = entry.cancelled.unwrap_or(false);
    let copy_stats = entry.copy_stats.unwrap_or_default();

    let new_run = NewExportRun {
        project_id,
        started_at: timestamp,
        // Legacy history records only a single completion timestamp per entry, never
        // a separate start time; both columns get the same value.
        finished_at: Some(timestamp),
        profile: entry.profile,
        safe_mode: entry.safe_export_mode,
        diff_mode: entry.diff_export_mode,
        files_copied: copy_stats.files_copied,
        // Legacy history never persisted a total-bytes figure on the run itself
        // (only indirectly, inside `snapshot_stats.bytes`, which duplicates the
        // snapshot payload's own sum and is not imported as a separate value).
        bytes_total: None,
        tokens_est: entry.tokens,
        // Legacy history never recorded a redaction count.
        redacted_count: None,
        cancelled,
        result_path: entry.result,
    };

    let archive_parts: Vec<NewArchivePart> = entry
        .archives
        .iter()
        .enumerate()
        .map(|(index, archive_name)| NewArchivePart {
            part_index: index as i64,
            archive_name: archive_name.clone(),
            compressed_bytes: None,
            groups: None,
        })
        .collect();

    // `entry.snapshot` is `None` only when the JSON key is entirely absent; a present
    // `{}` (the poisoned-empty-snapshot legacy bug — see `super`'s module docs)
    // deserializes to `Some(<empty map>)` and is imported faithfully as a real
    // snapshot row with zero `snapshot_file` rows, not dropped and not "fixed".
    let snapshot = entry.snapshot.map(|files| {
        let mut snapshot_files = Vec::with_capacity(files.len());
        let mut bytes_total = 0i64;
        for (rel_path, meta) in files {
            let size_bytes = meta.size.unwrap_or(0);
            bytes_total += size_bytes;
            snapshot_files.push(NewSnapshotFile {
                rel_path,
                sha256: meta.sha256.unwrap_or_default(),
                size_bytes,
                loc: meta.loc.unwrap_or(0),
                // Legacy history's per-file snapshot payload (`history_snapshot_payload`
                // in `diff_service.py`) only ever wrote `sha256`/`size`/`loc` — it never
                // persisted `mtime_ns`.
                mtime_ns: None,
            });
        }
        let new_snapshot = NewSnapshot {
            created_at: timestamp,
            file_count: snapshot_files.len() as i64,
            bytes_total,
        };
        (new_snapshot, snapshot_files)
    });
    let snapshot_ref = snapshot
        .as_ref()
        .map(|(new_snapshot, files)| (new_snapshot.clone(), files.as_slice()));

    record_export_run(conn, new_run, &[], &[], &archive_parts, snapshot_ref)?;

    Ok(ImportOutcome::Imported)
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.is_empty())
}

/// Parses legacy's `datetime.now().isoformat(sep=" ", timespec="seconds")` format
/// (`"YYYY-MM-DD HH:MM:SS"`, no timezone) into a Unix epoch timestamp, treating it as
/// UTC. Legacy's own timestamp carries no timezone information either — this import
/// cannot recover the offset that was actually in effect when the entry was written,
/// so imported `started_at`/`finished_at` values are a documented approximation, not
/// an exact reconstruction.
pub(super) fn parse_naive_datetime_to_unix(text: &str) -> Option<i64> {
    let (date_part, time_part) = text.split_once(' ')?;
    let mut date_fields = date_part.split('-');
    let year: i64 = date_fields.next()?.parse().ok()?;
    let month: u32 = date_fields.next()?.parse().ok()?;
    let day: u32 = date_fields.next()?.parse().ok()?;
    if date_fields.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let mut time_fields = time_part.split(':');
    let hour: i64 = time_fields.next()?.parse().ok()?;
    let minute: i64 = time_fields.next()?.parse().ok()?;
    let second: i64 = time_fields.next()?.parse().ok()?;
    if time_fields.next().is_some()
        || !(0..24).contains(&hour)
        || !(0..60).contains(&minute)
        || !(0..60).contains(&second)
    {
        return None;
    }

    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second)
}

/// Howard Hinnant's `days_from_civil` algorithm (public domain): the number of days
/// since the Unix epoch (1970-01-01) for a proleptic-Gregorian civil date. Correct
/// for the full range legacy timestamps can realistically take (modern years only).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_index = (i64::from(month) + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_exact_legacy_timespec_seconds_format() {
        assert_eq!(parse_naive_datetime_to_unix("1970-01-01 00:00:00"), Some(0));
        assert_eq!(
            parse_naive_datetime_to_unix("2024-01-02 03:04:05"),
            Some(1_704_164_645)
        );
    }

    #[test]
    fn rejects_malformed_timestamps() {
        assert_eq!(parse_naive_datetime_to_unix("not-a-date"), None);
        assert_eq!(parse_naive_datetime_to_unix("2024-13-40 99:99:99"), None);
    }
}
