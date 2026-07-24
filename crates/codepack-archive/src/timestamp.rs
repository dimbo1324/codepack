//! `generated_at` timestamp formatting, duplicated from
//! `codepack-scanner::plan::timestamp` (and `codepack-security::scan::write::timestamp`)
//! for the same out-of-scope-dependency reason (Q7, `docs/decisions/open-questions.md`):
//! this crate cannot depend on `codepack-scanner`/`codepack-security`. Renders a UTC
//! timestamp rather than legacy's local wall clock — a deliberate, documented
//! deviation for a cosmetic, non-contractual field (nothing parses `generated_at`
//! back).

use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn current_timestamp_utc() -> String {
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
    fn current_timestamp_has_the_expected_shape() {
        let stamp = current_timestamp_utc();
        assert!(stamp.ends_with(" UTC"));
        assert_eq!(stamp.len(), "2024-01-01 12:34:56 UTC".len());
    }
}
