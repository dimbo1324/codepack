//! Text/binary classification, ported from legacy `utils/text_utils.py`:
//! `should_consider_text_file` (extension/filename only, no content read) and
//! `looks_binary` (content sniffing on a bounded sample).

use std::path::Path;

use crate::constants;

/// Legacy's `_BINARY_SAMPLE_BYTES`: only the first 8192 bytes of a file are sniffed.
pub const BINARY_SAMPLE_BYTES: usize = 8192;

/// Extension/filename-only classification — never reads file content.
///
/// Order matters and is load-bearing (ported verbatim from
/// `should_consider_text_file`): a name match short-circuits before the extension is
/// even consulted, and a binary-extension match is checked before a text-extension
/// match.
pub fn should_consider_text_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let suffix = path
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if constants::is_text_filename_without_extension(&name) {
        return true;
    }
    if constants::is_binary_extension(&suffix) {
        return false;
    }
    if constants::is_text_extension(&suffix) {
        return true;
    }
    false
}

/// Content-sniffing heuristic on the first [`BINARY_SAMPLE_BYTES`] of `raw`: a NUL
/// byte anywhere in the sample means binary; otherwise a file is binary when more than
/// 30% of the sampled bytes are control bytes (`< 9` or strictly between 13 and 32,
/// i.e. excluding tab, newline, and carriage return).
pub fn looks_binary(raw: &[u8]) -> bool {
    let sample_len = raw.len().min(BINARY_SAMPLE_BYTES);
    let sample = &raw[..sample_len];

    if sample.contains(&0u8) {
        return true;
    }
    if sample.is_empty() {
        return false;
    }

    let control_bytes = sample
        .iter()
        .filter(|&&byte| byte < 9 || (13 < byte && byte < 32))
        .count();
    (control_bytes as f64 / sample.len() as f64) > 0.30
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_filename_without_extension_wins_first() {
        assert!(should_consider_text_file(Path::new("Dockerfile")));
        assert!(should_consider_text_file(Path::new(".env")));
        assert!(should_consider_text_file(Path::new("README")));
        assert!(should_consider_text_file(Path::new("LICENSE")));
    }

    #[test]
    fn binary_extension_beats_no_match() {
        assert!(!should_consider_text_file(Path::new("photo.PNG")));
        assert!(!should_consider_text_file(Path::new("archive.zip")));
    }

    #[test]
    fn text_extension_matches() {
        assert!(should_consider_text_file(Path::new("main.rs")));
        assert!(should_consider_text_file(Path::new("index.TS")));
    }

    #[test]
    fn unknown_extension_defaults_to_binary() {
        assert!(!should_consider_text_file(Path::new("mystery.xyz123")));
        assert!(!should_consider_text_file(Path::new("no_extension_at_all")));
    }

    #[test]
    fn readme_with_extension_uses_extension_rule_not_filename_rule() {
        // "readme.md" as a whole filename is not in TEXT_FILENAMES_WITHOUT_EXTENSION,
        // but "md" is in TEXT_EXTENSIONS, so it is still text via the third branch.
        assert!(should_consider_text_file(Path::new("README.md")));
    }

    #[test]
    fn looks_binary_detects_nul_byte() {
        assert!(looks_binary(b"hello\x00world"));
    }

    #[test]
    fn looks_binary_empty_sample_is_not_binary() {
        assert!(!looks_binary(b""));
    }

    #[test]
    fn looks_binary_plain_text_is_not_binary() {
        assert!(!looks_binary(
            b"the quick brown fox\njumps over\r\n\tthe lazy dog"
        ));
    }

    #[test]
    fn looks_binary_above_threshold_control_bytes() {
        // 4/10 = 0.40 > 0.30 -> binary.
        let mut sample = vec![b'a'; 6];
        sample.extend(std::iter::repeat_n(0x01u8, 4));
        assert!(looks_binary(&sample));
    }

    #[test]
    fn looks_binary_at_or_below_threshold_control_bytes() {
        // 3/10 = 0.30, not strictly greater than 0.30 -> not binary.
        let mut sample = vec![b'a'; 7];
        sample.extend(std::iter::repeat_n(0x01u8, 3));
        assert!(!looks_binary(&sample));
    }

    #[test]
    fn looks_binary_only_samples_first_8192_bytes() {
        let mut raw = vec![b'a'; BINARY_SAMPLE_BYTES];
        raw.push(0u8);
        assert!(!looks_binary(&raw));
    }
}
