//! Characterization test (audit 2026-09-07, `04-TESTS.txt` adversarial item 12):
//! cancelling while a single very large file is being packed into an archive is not
//! interruptible until that file finishes. This is known debt, recorded honestly in
//! `docs/architecture/overview.md`'s Known Debt list rather than fixed here — the
//! audit's own instruction is that a test fixing the current behavior is worth writing
//! *before* someone starts fixing it, so the change is visible later.
//!
//! Both writers (`zip_writer.rs`, `sevenz.rs`) check `cancel.is_cancelled()` exactly
//! once per member, *before* opening it — never while `std::io::copy` (ZIP) or
//! `push_archive_entry` (7z) is streaming that member's bytes. With a single member and
//! no member after it, there is no second checkpoint for a mid-copy cancellation to be
//! caught at, so the run completes as if cancellation was never requested. `#[ignore]`-
//! gated per `.ai/project/12-domain-rules.md`'s opt-in rule for heavy tests, run in the
//! same weekly job as `large_file_scan` (`.github/workflows/perf-smoke-weekly.yml`).

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use codepack_archive::{ArchiveFormat, pack_files};
use codepack_core::CancellationToken;

/// Large enough that `io::copy`/`push_archive_entry` runs through many internal
/// buffer-sized iterations rather than finishing in one, so a future fix that checks
/// cancellation every few chunks — rather than only between members — would have a real
/// window to interrupt inside, and this test would then need updating rather than
/// passing by accident.
const HUGE_FILE_BYTES: u64 = 64 * 1024 * 1024;
const WRITE_CHUNK_BYTES: usize = 1024 * 1024;

/// A cheap, deterministic, non-trivially-compressible fill: real large files (images,
/// archives, binaries) do not compress away to almost nothing the way repeated bytes or
/// text would, and a writer that finishes suspiciously fast would narrow the window a
/// background cancel has to land inside the copy rather than before or after it.
fn pseudo_random_chunk(bytes: usize, seed: &mut u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes);
    while out.len() < bytes {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        out.extend_from_slice(&seed.to_le_bytes());
    }
    out.truncate(bytes);
    out
}

fn write_huge_file(path: &std::path::Path) {
    let mut file = std::fs::File::create(path).unwrap();
    let mut seed = 0x9E3779B97F4A7C15u64;
    let mut written = 0u64;
    while written < HUGE_FILE_BYTES {
        let remaining = (HUGE_FILE_BYTES - written) as usize;
        let chunk = pseudo_random_chunk(remaining.min(WRITE_CHUNK_BYTES), &mut seed);
        file.write_all(&chunk).unwrap();
        written += chunk.len() as u64;
    }
}

#[test]
#[ignore = "writes a 64 MiB fixture file and packs it twice; run explicitly or in the weekly job"]
fn cancelling_mid_copy_of_the_sole_member_does_not_interrupt_it() {
    for format in [ArchiveFormat::Zip, ArchiveFormat::SevenZip] {
        let source = tempfile::tempdir().unwrap();
        let huge = source.path().join("huge.bin");
        write_huge_file(&huge);

        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join(format!("bundle.{}", format.extension()));
        let cancel = CancellationToken::new();

        // Requested from another thread shortly after the call starts, so it lands
        // after the loop's one and only pre-member check has already passed (that
        // check runs essentially at time zero) and, for a file this size, almost
        // certainly while the copy itself is still streaming.
        let cancel_from_elsewhere = cancel.clone();
        let canceller = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(5));
            cancel_from_elsewhere.cancel();
        });

        let result = pack_files(
            source.path(),
            &[PathBuf::from("huge.bin")],
            &archive,
            format,
            &cancel,
        );
        canceller.join().unwrap();

        assert!(
            result.is_ok(),
            "{format:?}: a cancellation requested during the sole member's copy was \
             expected to be ignored (known debt) but the run was interrupted instead — \
             if this is now intentional, update this test and the Known Debt entry in \
             docs/architecture/overview.md together, do not just delete the assertion"
        );
        assert!(archive.is_file(), "{format:?}");
    }
}
