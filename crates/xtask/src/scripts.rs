//! The Python half of the toolchain: the `scripts/` orchestrator's own test suite.
//!
//! `scripts/clean_project` deletes files. Its protected-path logic is the only thing
//! standing between "reduce the tree to what git tracks" and "delete a developer's live
//! credentials", and it has already failed that job twice in one task — once because
//! `lstrip("./")` strips characters rather than a prefix, once because git reports an
//! untracked directory as a single entry and the protection list was asked about the
//! directory's name instead of its contents. Both were found by hand.
//!
//! Tests for both exist. Until this module they ran in no gate and no CI, so the same
//! class of bug could reach `main` with everything green. That is the gap this closes.
//!
//! Optional on a developer machine, mandatory in CI — the same rule as [`crate::frontend`],
//! and for the same reason: a skip that is invisible in CI is a check that does not exist.

use std::path::Path;
use std::process::Command;

/// The interpreter name that actually resolves. Windows ships `python`; most Linux and
/// macOS distributions leave `python` either absent or pointing at Python 2, so `python3`
/// is tried first there.
const CANDIDATES: &[&str] = if cfg!(windows) {
    &["python", "python3"]
} else {
    &["python3", "python"]
};

fn interpreter() -> Option<&'static str> {
    CANDIDATES.iter().copied().find(|name| {
        Command::new(name)
            .arg("--version")
            .output()
            .is_ok_and(|out| out.status.success())
    })
}

/// Runs the orchestrator's unittest suite, or explains why it did not.
///
/// `-W error` is deliberate: an invalid escape sequence in a test file is a warning today
/// and a hard error in a future Python, and one had already slipped into the protection
/// tests. A warning nobody reads is how that reaches a release.
pub(crate) fn gate_checks(root: &Path) -> Result<(), String> {
    let Some(python) = interpreter() else {
        if std::env::var_os("CI").is_some() {
            return Err(
                "no Python interpreter found, and CI is set: the scripts/ test suite \
                 guards a tool that deletes files, so skipping it here would mean it \
                 runs nowhere"
                    .to_string(),
            );
        }
        println!(
            "  skipped: no Python interpreter on PATH.\n  \
             Install Python 3 to include the scripts/ test suite in this command."
        );
        return Ok(());
    };

    let ok = Command::new(python)
        .current_dir(root)
        .args([
            "-W", "error", "-m", "unittest", "discover", "-s", "scripts", "-t", ".",
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if ok {
        Ok(())
    } else {
        Err("scripts".to_string())
    }
}
