//! Timestamp formatting duplicated from `codepack-scanner::plan::timestamp` (and
//! `codepack-archive::timestamp`, `codepack-security::scan::write::timestamp`) for the
//! same reason those crates duplicated it from each other: this crate's own timestamp
//! needs (the export-bundle name stamp below) are small enough that pulling in a
//! calendar/timezone dependency, or a cross-crate dependency just for one helper, is
//! not worth it (same duplication-over-dependency tradeoff as Q7,
//! `docs/decisions/open-questions.md`, applied to a different duplicated helper).
//!
//! Legacy's `now_stamp()` formats the *local* wall clock
//! (`datetime.now().strftime("%Y%m%d_%H%M%S")`). Reproducing that would need a
//! timezone-database dependency; this renders a UTC timestamp instead, the same
//! deliberate, documented deviation already accepted for `ExportPlan.generated_at`
//! (`ROADMAP.md` S2 section) — the bundle-name stamp is not parsed back by anything,
//! only used for uniqueness and human orientation.

use std::time::{SystemTime, UNIX_EPOCH};

/// Compact `YYYYMMDD_HHMMSS` UTC stamp, ported shape from legacy `now_stamp()`.
pub(crate) fn compact_utc_stamp() -> String {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format_unix_seconds_compact(since_epoch.as_secs())
}

fn format_unix_seconds_compact(total_seconds: u64) -> String {
    let days = total_seconds / 86_400;
    let seconds_of_day = total_seconds % 86_400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}{month:02}{day:02}_{hour:02}{minute:02}{second:02}")
}

/// Howard Hinnant's `civil_from_days`: converts a day count since the Unix epoch
/// (1970-01-01) into a proleptic-Gregorian `(year, month, day)`. Avoids pulling in a
/// calendar/timezone crate for a single cosmetic timestamp field.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_formats_as_compact_stamp() {
        assert_eq!(format_unix_seconds_compact(0), "19700101_000000");
    }

    #[test]
    fn known_date_formats_correctly() {
        // 2024-01-01T12:34:56Z.
        assert_eq!(
            format_unix_seconds_compact(1_704_112_496),
            "20240101_123456"
        );
    }

    #[test]
    fn current_stamp_has_the_expected_shape() {
        let stamp = compact_utc_stamp();
        assert_eq!(stamp.len(), "19700101_000000".len());
        assert_eq!(stamp.chars().nth(8), Some('_'));
    }
}
