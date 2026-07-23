//! `ExportPlan.generated_at` formatting. Legacy's `human_now()` formats the *local*
//! wall clock (`datetime.now().isoformat(sep=" ", timespec="seconds")`); reproducing
//! that would need a timezone-database dependency, which is out of scope for this
//! stage's dependency list. This renders a UTC timestamp instead — a deliberate,
//! documented deviation for a cosmetic, non-contractual field (unlike
//! `PlannedFile`/`ExportPlan`'s other fields, nothing parses `generated_at` back).

use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn current_timestamp_utc() -> String {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format_unix_seconds(since_epoch.as_secs())
}

fn format_unix_seconds(total_seconds: u64) -> String {
    let days = total_seconds / 86_400;
    let seconds_of_day = total_seconds % 86_400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;

    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC")
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
    fn epoch_formats_as_1970_01_01() {
        assert_eq!(format_unix_seconds(0), "1970-01-01 00:00:00 UTC");
    }

    #[test]
    fn known_date_formats_correctly() {
        // 2024-01-01T12:34:56Z.
        assert_eq!(
            format_unix_seconds(1_704_112_496),
            "2024-01-01 12:34:56 UTC"
        );
    }

    #[test]
    fn current_timestamp_has_the_expected_shape() {
        let stamp = current_timestamp_utc();
        assert!(stamp.ends_with(" UTC"));
        assert_eq!(stamp.len(), "2024-01-01 12:34:56 UTC".len());
    }
}
