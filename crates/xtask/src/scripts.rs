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

    let output = Command::new(python)
        .current_dir(root)
        .args([
            "-W", "error", "-m", "unittest", "discover", "-s", "scripts", "-t", ".",
        ])
        .output()
        .map_err(|error| format!("dev scripts: could not run {python}: {error}"))?;

    // unittest reports its summary on stderr.
    let report = String::from_utf8_lossy(&output.stderr).into_owned()
        + &String::from_utf8_lossy(&output.stdout);
    print!("{report}");

    if !output.status.success() {
        return Err("dev scripts".to_string());
    }
    enforce_floor(&report)
}

/// The suite must not shrink silently.
///
/// `unittest discover` needs an `__init__.py` in every test directory and exits 0 when it
/// finds nothing. Deleting one file therefore removed all 23 clean-project tests — every
/// protected-path test this step exists for — while the gate stayed green. A step that
/// passes by not running is the exact failure this module was added to prevent, one layer
/// down.
///
/// The floor is a lower bound, not the current count: adding tests must never require
/// touching this, and removing the suite must always break the build.
const MINIMUM_TESTS: usize = 25;

fn enforce_floor(report: &str) -> Result<(), String> {
    let counted = report
        .lines()
        .filter_map(|line| line.strip_prefix("Ran "))
        .filter_map(|rest| rest.split_whitespace().next())
        .filter_map(|count| count.parse::<usize>().ok())
        .max();

    match counted {
        Some(count) if count >= MINIMUM_TESTS => Ok(()),
        Some(count) => Err(format!(
            "dev scripts: only {count} tests ran, expected at least {MINIMUM_TESTS}. \
             `unittest discover` exits 0 when it finds nothing, so a missing \
             `tests/__init__.py` silently drops a whole suite"
        )),
        None => Err(
            "dev scripts: could not read a test count from the unittest output, so there \
             is no evidence the suite ran"
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{MINIMUM_TESTS, enforce_floor};

    #[test]
    fn a_full_run_passes() {
        assert!(enforce_floor(&format!("Ran {MINIMUM_TESTS} tests in 2.0s\n\nOK\n")).is_ok());
    }

    #[test]
    fn a_silently_shrunken_suite_fails() {
        let error = enforce_floor("Ran 5 tests in 0.1s\n\nOK\n").unwrap_err();
        assert!(error.contains("only 5 tests ran"), "{error}");
    }

    #[test]
    fn output_without_a_count_fails_rather_than_passing() {
        assert!(enforce_floor("OK\n").is_err());
    }
}
