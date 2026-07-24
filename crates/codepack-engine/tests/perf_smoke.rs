//! Large-fixture performance smoke test (`task-checklist.md`'s S9 Verification
//! section), `#[ignore]`-gated per `.ai/project/12-domain-rules.md`'s expectation that
//! heavy tests are opt-in, not part of the default fast suite. Run explicitly with:
//!
//! ```text
//! cargo test -p codepack-engine --release -- --ignored large_fixture
//! ```
//!
//! This test's purpose is "does the full eight-step pipeline complete at ~50k files
//! without falling over" — a quadratic-blowup bug, an unbounded in-memory `Vec` where a
//! stream would do, a pathological per-file syscall count — not precise benchmarking.
//! The wall-clock budget below is a generous, clearly-labeled smoke ceiling, not a
//! tuned performance SLA: this project has never established one.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_storage::open;

const FILE_COUNT: usize = 50_000;
const FILES_PER_DIR: usize = 200;
// Measured on this pass's own sandbox (a virtualized, possibly antivirus-throttled
// Windows environment — `.ai/project/11-commands.md`'s own documented platform note):
// 5,000 files in ~12.9s, 50,000 files in ~155.0s — a ~12x wall-clock increase for a
// 10x file-count increase, consistent with linear scaling plus a modest constant
// per-run overhead, not a quadratic blowup (which would have shown roughly 100x for
// 10x). 300s gives ~2x headroom over the slower, real, measured run on this hardware.
const BUDGET: Duration = Duration::from_secs(300);

fn build_large_fixture(root: &Path) {
    let mut created = 0usize;
    let mut dir_index = 0usize;
    while created < FILE_COUNT {
        // Nested a few directories deep (not one flat directory of 50k entries, which
        // some filesystems handle poorly) — `dir_index` fans out `a/b/c`-shaped paths.
        let a = dir_index / 400;
        let b = (dir_index / 20) % 20;
        let c = dir_index % 20;
        let dir = root
            .join(format!("d{a}"))
            .join(format!("d{b}"))
            .join(format!("d{c}"));
        fs::create_dir_all(&dir).unwrap();
        for file_in_dir in 0..FILES_PER_DIR {
            if created >= FILE_COUNT {
                break;
            }
            fs::write(dir.join(format!("f{file_in_dir}.txt")), "line\n").unwrap();
            created += 1;
        }
        dir_index += 1;
    }
}

#[test]
#[ignore = "slow: generates and exports a ~50k-file fixture; run explicitly with --ignored"]
fn large_fixture_exports_within_a_generous_smoke_budget() {
    let source = tempfile::tempdir().unwrap();
    build_large_fixture(source.path());

    let output = tempfile::tempdir().unwrap();
    let db_dir = tempfile::tempdir().unwrap();
    let mut conn = open(&db_dir.path().join("codepack.db")).unwrap();
    let config = Config::default();
    let (tx, _rx) = codepack_core::progress_channel();

    let started = Instant::now();
    let outcome = codepack_engine::run_export(
        &mut conn,
        source.path(),
        output.path(),
        &config,
        &HashMap::new(),
        &tx,
        &CancellationToken::new(),
    )
    .unwrap();
    let elapsed = started.elapsed();

    eprintln!(
        "large_fixture_exports_within_a_generous_smoke_budget: {} files, elapsed = {elapsed:?}",
        FILE_COUNT
    );

    assert!(outcome.successful, "copy_stats = {:?}", outcome.copy_stats);
    assert_eq!(outcome.copy_stats.errors, 0);
    let primary = outcome
        .archive_result
        .primary_result()
        .expect("a successful run always produces a primary archive result");
    assert!(primary.exists());

    assert!(
        elapsed < BUDGET,
        "export of {FILE_COUNT} files took {elapsed:?}, exceeding the {BUDGET:?} smoke \
         ceiling (a smoke ceiling, not a tuned SLA — see this file's own module doc \
         comment)"
    );
}
