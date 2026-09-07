//! Heavy fixture test (audit 2026-09-07, P-2/Q-3; `04-TESTS.txt` item 10): a file past
//! the scanner's hard read ceiling must still let the scan complete, rather than reading
//! the whole thing into memory. `#[ignore]`-gated per `.ai/project/12-domain-rules.md`'s
//! expectation that heavy tests are opt-in, not part of the default fast suite — the
//! same convention `codepack-engine`'s `perf_smoke.rs` follows. Run explicitly with:
//!
//! ```text
//! cargo test -p codepack-security --release --test large_file_scan -- --ignored
//! ```
//!
//! Before this pass, `scan_one_file` read a candidate text file whole regardless of
//! size whenever `max_bytes_per_file` was unset — which is always true for `codepack
//! scan`, the command this test stands in for, since it forces that limit off on
//! purpose (it answers "does this project contain a secret", not "would this fit in an
//! export"). A multi-hundred-megabyte file, scanned in parallel with others on every
//! `rayon` worker at once, was the unbounded-memory failure mode this fixture exercises.

use std::io::Write;

use codepack_core::CancellationToken;
use codepack_security::{FindingKind, scan_project};

/// Comfortably past `codepack_core::classify::ABSOLUTE_MAX_TEXT_FILE_READ_BYTES` (256
/// MiB), matching the audit's own "a few hundred megabytes" — large enough that reading
/// it whole, rather than up to the ceiling, is the failure this test would have caught.
const HUGE_FILE_BYTES: u64 = 300 * 1024 * 1024;

/// Written in chunks rather than built as one in-memory `Vec` first: the fixture's own
/// construction must not be the thing that uses gigabytes of memory.
const WRITE_CHUNK_BYTES: usize = 1024 * 1024;

#[test]
#[ignore = "writes a 300 MiB fixture file; run explicitly or in the weekly job"]
fn a_multi_hundred_megabyte_file_is_scanned_up_to_the_ceiling_not_skipped_or_read_whole() {
    let dir = tempfile::tempdir().unwrap();
    let huge = dir.path().join("database_dump.log");

    {
        let mut file = std::fs::File::create(&huge).unwrap();
        // Realistic text content, not a single repeated byte, so `looks_binary`'s NUL
        // sniff and the line-based detectors below see what a real log looks like — a
        // line long enough that `.lines()` over the scanned prefix does real work
        // without needing millions of tiny allocations to reach 300 MiB.
        let line = "2026-09-07T00:00:00Z INFO request handled ok, nothing to see here\n";
        let chunk = line.repeat(WRITE_CHUNK_BYTES / line.len() + 1);
        let mut written = 0u64;
        while written < HUGE_FILE_BYTES {
            let remaining = (HUGE_FILE_BYTES - written) as usize;
            let slice = &chunk.as_bytes()[..remaining.min(chunk.len())];
            file.write_all(slice).unwrap();
            written += slice.len() as u64;
        }
    }
    let total_bytes = std::fs::metadata(&huge).unwrap().len();
    assert!(total_bytes >= HUGE_FILE_BYTES);

    let files = vec![std::path::PathBuf::from("database_dump.log")];
    let result = scan_project(dir.path(), &files, None, &CancellationToken::new())
        .expect("scanning a huge file must complete, not error out or hang");

    assert_eq!(
        result.summary.partial_scans, 1,
        "the file exceeds the ceiling and must be reported as partially scanned"
    );
    let partial = result
        .findings
        .iter()
        .find(|finding| finding.kind == FindingKind::PartialScan)
        .expect("a PartialScan finding must be present");
    assert!(partial.file.ends_with("database_dump.log"));
    assert!(
        partial.message.contains(&total_bytes.to_string()),
        "the finding should name the file's real size: {}",
        partial.message
    );
}
