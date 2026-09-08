//! The scan cache the pipeline hands to `codepack-security`, backed by SQLite.
//!
//! This is the crate that legitimately depends on both, so this is where the two meet:
//! `codepack-security` decides what may be reused and under which key,
//! `codepack-storage` keeps the rows, and neither knows about the other.
//!
//! ## Why the whole table is read up front
//!
//! Scanning runs on a rayon pool and a `rusqlite::Connection` is not `Sync`, so a worker
//! thread cannot query. The table is therefore loaded into a map before the pass and
//! consulted from memory; everything learned during the pass — entries that were not
//! there, and keys that were used — is buffered and written once afterwards.
//!
//! Entries are small (most files contain nothing, and their entry says so in two
//! characters), and the ceiling below bounds the rest.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use codepack_security::cache::{self, CachedFinding, FileScanCache};
use codepack_storage::Connection;

use crate::error::Result;

/// How many entries survive a prune. Chosen to comfortably cover a large repository's
/// file count several times over while keeping the load cost trivial.
const MAX_ENTRIES: u32 = 50_000;

pub(crate) struct SqliteScanCache {
    entries: HashMap<String, Vec<CachedFinding>>,
    /// Newly scanned content, `(key, findings_json)`, waiting to be written.
    pending: Mutex<Vec<(String, String)>>,
    /// Keys that were served from `entries`, so pruning keeps what is in use.
    ///
    /// A set, not a list. The same key is served once per file with those bytes, and a
    /// repository full of identical small files (a vendored dependency, a generated
    /// header) hit the same key thousands of times — each one pushing a fresh 64-character
    /// `String` that the flush then turned into its own `UPDATE` (audit No. 19).
    used: Mutex<HashSet<String>>,
}

impl SqliteScanCache {
    /// Reads the cache into memory.
    ///
    /// A row whose JSON no longer parses is skipped rather than fatal: it was written by
    /// a different build, and the worst it can cost is one file being scanned again.
    pub(crate) fn load(conn: &Connection) -> Result<Self> {
        let mut entries = HashMap::new();
        codepack_storage::scan_cache::load_scan_cache(conn, |key, json| {
            if let Some(findings) = cache::decode(&json) {
                entries.insert(key, findings);
            }
        })?;
        Ok(Self {
            entries,
            pending: Mutex::new(Vec::new()),
            used: Mutex::new(HashSet::new()),
        })
    }

    /// Writes everything this run learned, then trims the cache back to its ceiling.
    pub(crate) fn flush(self, conn: &mut Connection) -> Result<()> {
        let pending = into_inner(self.pending);
        let used: Vec<String> = into_inner(self.used).into_iter().collect();
        codepack_storage::scan_cache::store_scan_cache(conn, &pending, &used)?;
        codepack_storage::scan_cache::prune_scan_cache(conn, MAX_ENTRIES)?;
        Ok(())
    }
}

/// Takes a mutex's value, treating poisoning as recoverable.
///
/// A panic on a worker thread must not turn a cache — an optimisation, by definition
/// discardable — into a failure of the export it was meant to speed up.
fn into_inner<T>(lock: Mutex<T>) -> T {
    lock.into_inner().unwrap_or_else(|error| error.into_inner())
}

impl FileScanCache for SqliteScanCache {
    fn lookup(&self, key: &str) -> Option<Vec<CachedFinding>> {
        let found = self.entries.get(key)?;
        // Audit 2026-09-07, P-5: the same recovery `into_inner` above already applies
        // to `flush`, applied here too — this was the asymmetry the audit found. `Ok`
        // silently dropped every mark-as-used call the moment any rayon worker panicked
        // anywhere in the pass, which then made `prune_scan_cache` delete entries that
        // genuinely were in use because they were never marked so.
        lock(&self.used).insert(key.to_string());
        Some(found.clone())
    }

    fn store(&self, key: &str, findings: &[CachedFinding]) {
        // A finding that cannot be serialised is simply not cached; the scan already
        // produced the real answer, and this path exists only to make the next run
        // faster.
        let Some(json) = cache::encode(findings) else {
            return;
        };
        lock(&self.pending).push((key.to_string(), json));
    }
}

/// Locks a mutex, treating poisoning as recoverable — the borrowing twin of
/// [`into_inner`]. A panic on one rayon worker must not silently stop every other
/// worker's cache writes for the rest of the pass.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache_with_one_entry() -> (tempfile::TempDir, SqliteScanCache) {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = codepack_storage::open(&dir.path().join("codepack.db")).unwrap();
        let json = cache::encode(&[]).unwrap();
        codepack_storage::scan_cache::store_scan_cache(
            &mut conn,
            &[("planted-key".to_string(), json)],
            &[],
        )
        .unwrap();
        let cache = SqliteScanCache::load(&conn).unwrap();
        (dir, cache)
    }

    /// Audit 2026-09-07, P-5: before this fix, `lookup` on a poisoned `used` mutex
    /// silently stopped marking keys as used for the rest of the pass — and
    /// `prune_scan_cache` then deleted entries that genuinely were in use, because
    /// nothing ever recorded that they were.
    #[test]
    fn a_lookup_still_marks_the_key_used_after_the_mutex_is_poisoned() {
        let (_dir, cache) = cache_with_one_entry();
        poison_the_field(&cache.used);

        let found = cache.lookup("planted-key");
        assert!(found.is_some(), "the entry itself must still be served");
        assert!(
            lock(&cache.used).contains("planted-key"),
            "lookup must still record the key as used past a poisoned mutex"
        );
    }

    /// Same shape as the lookup test, for the other mutex `store` writes through.
    #[test]
    fn a_store_still_records_the_entry_after_the_mutex_is_poisoned() {
        let (_dir, cache) = cache_with_one_entry();
        poison_the_field(&cache.pending);

        cache.store("new-key", &[]);

        let pending = lock(&cache.pending);
        assert!(
            pending.iter().any(|(key, _)| key == "new-key"),
            "store must still record the entry past a poisoned mutex: {pending:?}"
        );
    }

    /// Poisons `field` in place, on the current process — no leaking, no swapping
    /// fields, just a panic on a scoped thread that borrows the mutex for exactly as
    /// long as it needs to poison it. `std::thread::scope` is what makes borrowing
    /// (rather than `'static`) sound here.
    fn poison_the_field<T: Send>(field: &Mutex<T>) {
        std::thread::scope(|scope| {
            let _ = scope
                .spawn(|| {
                    let _guard = field.lock().unwrap();
                    panic!("planted panic to poison the mutex, matching a rayon worker's own");
                })
                .join();
        });
        assert!(field.is_poisoned());
    }
}
