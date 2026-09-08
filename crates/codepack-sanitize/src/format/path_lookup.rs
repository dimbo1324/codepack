//! Manual `PATH` binary resolution.
//!
//! Not the `which` crate: this needs exactly one thing — "does this bare name resolve to
//! a runnable file on `PATH`" — and a dependency that also handles custom search paths,
//! cross-platform quirks and `PATHEXT` edge cases is more than one lookup needs
//! (`.ai/universal/05-security-and-secrets.md`). `std::process::Command::new` does not
//! answer the question up front; it only fails, per candidate, at spawn time.
//!
//! The two platforms search differently and both are implemented here. Windows appends
//! every `PATHEXT` extension, without which `rustfmt` never resolves at all. Unix looks
//! for the name itself and asks whether anyone may execute it — a `PATH` directory holds
//! plenty of files that are not programs. Getting this wrong is quiet: the caller reads a
//! `None` as "no formatter installed" and writes unformatted output, so a Unix-blind
//! lookup would have disabled this whole feature on macOS and Linux without one error
//! message (found on both Unix CI runners, 2026-09-06).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Finds `binary_name` on `PATH`, or `None` if nothing runnable answers to it.
///
/// Uncached: [`find_on_path`] is the one that memoizes. This is the real search, kept
/// separate so the cache itself stays a thin wrapper a reader does not have to untangle
/// from the search logic.
fn search_path(binary_name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        for name in candidate_names(binary_name) {
            let candidate = dir.join(name);
            if is_runnable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// Finds `binary_name` on `PATH`, memoized for the life of the process (audit
/// 2026-09-07, Q-11).
///
/// `format_source` calls this once per file, and every file of one language asks the
/// same finite, compile-time-known set of formatter names — a project of 5,000 files in
/// one language was 5,000 full walks of `PATH`, each one the same dozens of `metadata()`
/// system calls, all giving the same answer. Sanitizing is an on-demand command rather
/// than part of every export, so this was never on the hot path the way the security
/// scanner's own per-file work is, but it is exactly the "walk a list per element" shape
/// a perf review looks for, and cheaper to fix than to keep explaining.
///
/// **The one honest caveat, worth its own line rather than leaving a reader to
/// rediscover it:** a process-wide cache means installing a formatter (or fixing its
/// permissions) while the desktop app is already running goes unnoticed until restart.
/// Acceptable for a desktop application — nothing here is expected to change PATH out
/// from under a long-running process — and cheaper than re-checking every file just in
/// case it did.
pub(super) fn find_on_path(binary_name: &'static str) -> Option<PathBuf> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, Option<PathBuf>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    let mut cache = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry(binary_name)
        .or_insert_with(|| search_path(binary_name))
        .clone()
}

/// The file names one `PATH` directory is searched for, in order.
///
/// On Unix a command is its own file name and nothing else. On Windows every `PATHEXT`
/// entry is appended instead, exactly as `CreateProcess`'s own search does — unless the
/// name already carries an extension, in which case it is taken as written.
fn candidate_names(binary_name: &str) -> Vec<String> {
    if !cfg!(windows) || Path::new(binary_name).extension().is_some() {
        return vec![binary_name.to_string()];
    }
    let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string());
    pathext
        .split(';')
        .filter(|ext| !ext.is_empty())
        .map(|ext| format!("{binary_name}{ext}"))
        .collect()
}

/// On Unix the executable bit is the question, since `PATH` directories hold data files
/// too and `is_file` alone would hand back a candidate that cannot be spawned.
#[cfg(unix)]
fn is_runnable(candidate: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    candidate
        .metadata()
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// Windows has no executable bit; `PATHEXT` already decided what counts as runnable.
#[cfg(not(unix))]
fn is_runnable(candidate: &Path) -> bool {
    candidate.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_binary_that_cannot_exist_is_not_found() {
        assert!(find_on_path("codepack-sanitize-definitely-not-a-real-tool-9f3b").is_none());
    }

    /// `search_path` (the uncached half `find_on_path` wraps) is a plain
    /// `fn(&str) -> Option<PathBuf>` with no shared state, so the get-or-insert shape in
    /// `find_on_path` itself — read directly, a dozen lines above — is what actually
    /// guarantees the real search runs at most once per name; there is no internal
    /// counter to assert against without adding test-only instrumentation to that
    /// function, which is more machinery than a low-priority cache justifies. What is
    /// worth a test, and testable honestly, is the property callers actually depend on:
    /// repeated lookups of the same name keep agreeing, including after the answer has
    /// presumably been served from the cache rather than a fresh search.
    #[test]
    fn repeated_lookups_of_the_same_binary_agree() {
        let always_present = if cfg!(windows) { "cmd" } else { "sh" };
        let first = find_on_path(always_present);
        let second = find_on_path(always_present);
        let third = find_on_path(always_present);
        assert!(first.is_some());
        assert_eq!(first, second);
        assert_eq!(second, third);
    }

    #[test]
    fn a_binary_every_supported_platform_has_resolves() {
        // The cheapest possible proof that the search walks real directories rather than
        // trivially returning `None` for everything. One name per platform, each
        // guaranteed present: `cmd.exe` ships with every Windows install, and `sh` is
        // POSIX. Naming only the Windows one is how this test passed on Windows while
        // the lookup it guards was broken on both Unix platforms.
        let always_present = if cfg!(windows) { "cmd" } else { "sh" };
        assert!(
            find_on_path(always_present).is_some(),
            "{always_present} should resolve on PATH"
        );
    }
}
