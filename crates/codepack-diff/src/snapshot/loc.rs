//! Line-count-if-text helper, ported from legacy `diff_service.py::_count_loc`. The
//! "is this a countable text file" predicate is caller-supplied (`Fn(&Path) -> bool`)
//! rather than this crate embedding its own copy of `TEXT_EXTENSIONS`/
//! `TEXT_FILENAMES_WITHOUT_EXTENSION` — those constants already live in
//! `codepack-scanner`/`codepack-security`, and this crate's scope boundary forbids a
//! third copy (see `lib.rs`).
//!
//! Legacy reads the whole file into memory for this pass (`path.read_text(...)`) —
//! only hashing (`hash.rs`) is streamed. That asymmetry is legacy behavior, not
//! something this stage introduces or is asked to fix.

use std::fs;
use std::path::Path;

pub fn count_loc_if_countable<F>(path: &Path, is_countable_text: &F) -> u64
where
    F: Fn(&Path) -> bool,
{
    if !is_countable_text(path) {
        return 0;
    }
    let Ok(bytes) = fs::read(path) else {
        return 0;
    };
    let text = String::from_utf8_lossy(&bytes);
    text.lines().filter(|line| !line.trim().is_empty()).count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn always_text(_: &Path) -> bool {
        true
    }

    fn never_text(_: &Path) -> bool {
        false
    }

    #[test]
    fn counts_non_blank_lines_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "line one\n\n  \nline two\nline three\n").unwrap();

        assert_eq!(count_loc_if_countable(&path, &always_text), 3);
    }

    #[test]
    fn returns_zero_when_predicate_rejects_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.bin");
        std::fs::write(&path, "line one\nline two\n").unwrap();

        assert_eq!(count_loc_if_countable(&path, &never_text), 0);
    }

    #[test]
    fn returns_zero_for_unreadable_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.txt");

        assert_eq!(count_loc_if_countable(&path, &always_text), 0);
    }
}
