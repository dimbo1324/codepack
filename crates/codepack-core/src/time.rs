//! UTC civil-time formatting: the single implementation shared by every crate that
//! stamps a generated artifact.
//!
//! ## Why this lives in `codepack-core`
//!
//! Six modules across five crates each carried their own byte-identical copy of Howard
//! Hinnant's `civil_from_days` plus the same `/86_400`, `/3600`, `%60` decomposition.
//! Each copy justified itself the same way — "this crate must not depend on that crate
//! for one small helper" — which was true pairwise and wrong in aggregate: every one of
//! those crates already depends on `codepack-core`, so the helper belongs here, where a
//! calendar bug can be fixed once instead of six times.
//!
//! ## Why hand-rolled rather than a calendar crate
//!
//! A timezone database would be needed to reproduce legacy's *local* wall clock; these
//! timestamps are cosmetic (nothing parses `generated_at` back), so every artifact
//! renders **UTC** instead. That is a deliberate, long-standing deviation from legacy,
//! recorded in `docs/__arch__/ROADMAP.md`'s S2 section and reaffirmed here — not a gap.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds in a day; the unit the epoch-day conversion counts in.
const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_MINUTE: i64 = 60;

/// Days from 0000-03-01 (the algorithm's internal era origin) to 1970-01-01.
/// Shifting by this turns a Unix epoch day count into the era-relative count Hinnant's
/// algorithm expects.
const DAYS_FROM_ERA_ORIGIN_TO_UNIX_EPOCH: i64 = 719_468;

/// Days in a 400-year Gregorian era — the cycle after which the calendar repeats
/// exactly (400 years = 146_097 days, leap rules included).
const DAYS_PER_ERA: i64 = 146_097;

/// A broken-down UTC civil date and time.
///
/// Constructing this once and formatting from it replaces the repeated inline
/// `total_seconds / 86_400` / `% 3600` / `% 60` arithmetic that used to appear at each
/// call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtcDateTime {
    pub year: i64,
    /// 1–12.
    pub month: u32,
    /// 1–31.
    pub day: u32,
    /// 0–23.
    pub hour: u32,
    /// 0–59.
    pub minute: u32,
    /// 0–59.
    pub second: u32,
}

impl UtcDateTime {
    /// Decomposes a Unix timestamp into UTC civil fields.
    ///
    /// Takes `i64` rather than `u64` because Git commit timestamps are signed and a
    /// repository may legitimately carry a pre-1970 author date. `div_euclid`/
    /// `rem_euclid` (not `/` and `%`) are what make that work: they floor toward
    /// negative infinity, so a negative timestamp yields a positive time-of-day on the
    /// preceding day rather than a negative hour.
    pub fn from_unix_seconds(total_seconds: i64) -> Self {
        let days = total_seconds.div_euclid(SECONDS_PER_DAY);
        let seconds_of_day = total_seconds.rem_euclid(SECONDS_PER_DAY);

        let (year, month, day) = civil_from_days(days);
        Self {
            year,
            month,
            day,
            hour: (seconds_of_day / SECONDS_PER_HOUR) as u32,
            minute: ((seconds_of_day % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE) as u32,
            second: (seconds_of_day % SECONDS_PER_MINUTE) as u32,
        }
    }

    /// Current wall-clock time as UTC civil fields. A system clock set before the Unix
    /// epoch yields the epoch itself rather than panicking — a self-evidently wrong
    /// timestamp in a cosmetic field is preferable to aborting an export.
    pub fn now() -> Self {
        Self::from_unix_seconds(unix_timestamp_now())
    }

    /// `YYYY-MM-DD HH:MM:SS` — the shape legacy's `human_now()` produced (in local
    /// time; see the module doc for why this renders UTC instead).
    pub fn format_human(&self) -> String {
        let (y, mo, d, h, mi, s) = self.parts();
        format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
    }

    /// [`Self::format_human`] with an explicit ` UTC` suffix, for artifact headers where
    /// a reader could otherwise mistake the value for local time.
    pub fn format_human_utc(&self) -> String {
        format!("{} UTC", self.format_human())
    }

    /// `YYYYMMDD_HHMMSS` — legacy's `now_stamp()` shape, used in generated file and
    /// bundle names where separators would be awkward.
    pub fn format_compact(&self) -> String {
        let (y, mo, d, h, mi, s) = self.parts();
        format!("{y:04}{mo:02}{d:02}_{h:02}{mi:02}{s:02}")
    }

    /// `YYYY-MM-DD` — date alone, for per-commit listings where the time adds noise.
    pub fn format_date(&self) -> String {
        let (y, mo, d, ..) = self.parts();
        format!("{y:04}-{mo:02}-{d:02}")
    }

    fn parts(&self) -> (i64, u32, u32, u32, u32, u32) {
        (
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
        )
    }
}

/// Howard Hinnant's `civil_from_days` (public domain): converts a day count since the
/// Unix epoch into a proleptic-Gregorian `(year, month, day)`.
///
/// The algorithm works in 400-year eras counted from March (`mp`, the "month of year"
/// index, is March-based) so that the leap day lands at the end of a year and needs no
/// special case; the final `if month <= 2` shifts January and February back into the
/// conventional calendar year. The integer constants are inherent to that formulation —
/// `153` and `5` encode the repeating 5-month/153-day pattern of March-based month
/// lengths — and are meaningful only together, which is why they are left inline rather
/// than named individually.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + DAYS_FROM_ERA_ORIGIN_TO_UNIX_EPOCH;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - (DAYS_PER_ERA - 1)
    } / DAYS_PER_ERA;
    let day_of_era = (shifted - era * DAYS_PER_ERA) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

/// Whole-second Unix epoch "now".
///
/// Falls back to `0` rather than panicking if the system clock is set before the Unix
/// epoch: a self-evidently wrong sentinel in a cosmetic field never justifies aborting
/// a running export.
pub fn unix_timestamp_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

/// Whole-second Unix epoch value for an already-known [`SystemTime`] — a file's mtime,
/// say. Pre-epoch times clamp to `0`, matching [`unix_timestamp_now`].
pub fn unix_seconds_of(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

/// `YYYY-MM-DD HH:MM:SS UTC` for the current moment — the default artifact-header stamp.
pub fn now_human_utc() -> String {
    UtcDateTime::now().format_human_utc()
}

/// `YYYY-MM-DD HH:MM:SS` for the current moment, without the ` UTC` suffix.
pub fn now_human() -> String {
    UtcDateTime::now().format_human()
}

/// `YYYYMMDD_HHMMSS` for the current moment — for generated names.
pub fn now_compact() -> String {
    UtcDateTime::now().format_compact()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_decomposes_to_1970_01_01() {
        let dt = UtcDateTime::from_unix_seconds(0);
        assert_eq!(
            (dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second),
            (1970, 1, 1, 0, 0, 0)
        );
    }

    #[test]
    fn known_timestamp_decomposes_correctly() {
        // 2024-01-01T12:34:56Z.
        let dt = UtcDateTime::from_unix_seconds(1_704_112_496);
        assert_eq!(
            (dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second),
            (2024, 1, 1, 12, 34, 56)
        );
    }

    #[test]
    fn all_four_formats_render_the_same_instant_consistently() {
        let dt = UtcDateTime::from_unix_seconds(1_704_112_496);
        assert_eq!(dt.format_human(), "2024-01-01 12:34:56");
        assert_eq!(dt.format_human_utc(), "2024-01-01 12:34:56 UTC");
        assert_eq!(dt.format_compact(), "20240101_123456");
        assert_eq!(dt.format_date(), "2024-01-01");
    }

    #[test]
    fn negative_timestamps_yield_a_positive_time_of_day() {
        // Git author dates are signed and a repository may carry a pre-epoch one.
        // Plain `/` and `%` would produce a negative hour here; `div_euclid`/
        // `rem_euclid` roll back into the previous day instead.
        let dt = UtcDateTime::from_unix_seconds(-1);
        assert_eq!(
            (dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second),
            (1969, 12, 31, 23, 59, 59)
        );
    }

    #[test]
    fn a_full_pre_epoch_day_decomposes_correctly() {
        // Exactly one day before the epoch: midnight on 1969-12-31.
        let dt = UtcDateTime::from_unix_seconds(-86_400);
        assert_eq!((dt.year, dt.month, dt.day), (1969, 12, 31));
        assert_eq!((dt.hour, dt.minute, dt.second), (0, 0, 0));
    }

    #[test]
    fn leap_day_and_the_day_after_are_both_correct() {
        // A hand-rolled civil calendar that is subtly wrong would corrupt every
        // timestamp silently, so the leap rules are pinned explicitly: a year divisible
        // by 4, a year divisible by 400, and the rollover immediately after a leap day.
        assert_eq!(civil_from_days(19_051), (2022, 2, 28));
        // 2024-02-29 (divisible by 4) and the day after.
        let leap_day = UtcDateTime::from_unix_seconds(1_709_164_800);
        assert_eq!((leap_day.year, leap_day.month, leap_day.day), (2024, 2, 29));
        let day_after = UtcDateTime::from_unix_seconds(1_709_164_800 + 86_400);
        assert_eq!(
            (day_after.year, day_after.month, day_after.day),
            (2024, 3, 1)
        );
        // 2000-02-29: divisible by 400, so a leap year despite being a century.
        let century_leap = UtcDateTime::from_unix_seconds(951_782_400);
        assert_eq!(
            (century_leap.year, century_leap.month, century_leap.day),
            (2000, 2, 29)
        );
        // 1900-03-01: 1900 is divisible by 100 but not 400, so it is *not* a leap year
        // and 1900-02-29 does not exist. Pre-epoch, which also exercises the negative
        // era branch.
        let non_leap_century = UtcDateTime::from_unix_seconds(-2_203_891_200);
        assert_eq!(
            (
                non_leap_century.year,
                non_leap_century.month,
                non_leap_century.day
            ),
            (1900, 3, 1)
        );
    }

    #[test]
    fn every_day_across_a_four_year_leap_cycle_round_trips() {
        // Property check over a contiguous range rather than spot values: walk day by
        // day through 2023-2026 and assert the date advances by exactly one calendar
        // day each step, with month and day always in range. This catches an off-by-one
        // anywhere in the era arithmetic that isolated known-value assertions could miss.
        let start_day = 19_358; // 2023-01-01
        let (mut prev_y, mut prev_m, mut prev_d) = civil_from_days(start_day);
        for offset in 1..=(4 * 366) {
            let (y, m, d) = civil_from_days(start_day + offset);
            assert!((1..=12).contains(&m), "month out of range: {y}-{m}-{d}");
            assert!((1..=31).contains(&d), "day out of range: {y}-{m}-{d}");

            let advanced_within_month = y == prev_y && m == prev_m && d == prev_d + 1;
            let rolled_to_next_month = y == prev_y && m == prev_m + 1 && d == 1;
            let rolled_to_next_year = y == prev_y + 1 && m == 1 && d == 1 && prev_m == 12;
            assert!(
                advanced_within_month || rolled_to_next_month || rolled_to_next_year,
                "non-contiguous step: {prev_y}-{prev_m}-{prev_d} -> {y}-{m}-{d}"
            );
            (prev_y, prev_m, prev_d) = (y, m, d);
        }
    }

    #[test]
    fn now_helpers_have_the_documented_shapes() {
        assert_eq!(now_human().len(), "2024-01-01 12:34:56".len());
        assert!(now_human_utc().ends_with(" UTC"));
        let compact = now_compact();
        assert_eq!(compact.len(), "20240101_123456".len());
        assert_eq!(compact.chars().nth(8), Some('_'));
    }

    #[test]
    fn unix_timestamp_now_is_a_plausible_recent_epoch_second() {
        // A loose lower bound (2023-11-14) that holds for the life of this codebase
        // without pinning a moving "current" value.
        assert!(unix_timestamp_now() > 1_700_000_000);
    }

    #[test]
    fn unix_seconds_of_clamps_pre_epoch_times_to_zero() {
        assert_eq!(unix_seconds_of(UNIX_EPOCH), 0);
        assert_eq!(
            unix_seconds_of(UNIX_EPOCH - std::time::Duration::from_secs(5)),
            0
        );
        assert_eq!(
            unix_seconds_of(UNIX_EPOCH + std::time::Duration::from_secs(1_704_112_496)),
            1_704_112_496
        );
    }
}
