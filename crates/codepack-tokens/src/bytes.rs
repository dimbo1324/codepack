//! Byte-size formatting (BLUEPRINT §E.1 basis; legacy `utils/text_utils.py::format_bytes`).

/// Formats a byte count using binary (1024) units, porting the legacy formatter's
/// loop/threshold structure verbatim (`utils/text_utils.py::format_bytes` in the
/// archived Python implementation).
///
/// Units are `B`, `KB`, `MB`, `GB`, `TB` — there is no petabyte tier. Once a value
/// reaches the `TB` tier it stays there no matter how large it grows: the loop's
/// terminal condition is "this is the last unit in the list", not a byte-count
/// ceiling, so the printed number keeps growing rather than rolling into an unlisted
/// `PB` tier.
///
/// The `B` tier always prints as a truncated integer with no decimals; every other
/// tier prints with exactly two decimal places, including boundary values whose
/// division remainder carries into the next tier's textual rounding — for example
/// `1024 * 1024 - 1` bytes prints as `"1024.00 KB"`, not `"1023.99 KB"`. That is the
/// legacy formatter's actual output (confirmed against the archive), reproduced
/// verbatim rather than "corrected".
pub fn format_bytes(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let last_index = UNITS.len() - 1;
    let mut value = size as f64;
    for (index, unit) in UNITS.into_iter().enumerate() {
        if value < 1024.0 || index == last_index {
            if unit == "B" {
                return format!("{} {unit}", value as u64);
            }
            return format!("{value:.2} {unit}");
        }
        value /= 1024.0;
    }
    unreachable!("\"TB\" is the last unit and its branch always returns from the loop")
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn zero_bytes_prints_as_bare_integer() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn just_under_one_kib_stays_in_bytes() {
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn exactly_one_kib_rolls_over() {
        assert_eq!(format_bytes(1024), "1.00 KB");
    }

    #[test]
    fn just_under_one_mib_carries_rounding_into_kib() {
        // 1024 * 1024 - 1 bytes / 1024 = 1023.9990234375 KB, which the legacy
        // formatter's `.2f` rounds up to "1024.00 KB" (not "1023.99 KB"). This is a
        // known legacy oddity at the boundary, ported verbatim rather than fixed.
        assert_eq!(format_bytes(1024 * 1024 - 1), "1024.00 KB");
    }

    #[test]
    fn exactly_one_tib() {
        assert_eq!(format_bytes(1024u64.pow(4)), "1.00 TB");
    }

    #[test]
    fn far_beyond_one_tib_stays_in_tb_with_no_pb_tier() {
        assert_eq!(format_bytes(1024u64.pow(5)), "1024.00 TB");
    }
}
