//! A durable, machine-readable record of one `cargo xtask gate` run (audit 2026-09-07,
//! G-2).
//!
//! Before this, nine of the gate's ten sections wrote straight to the console and kept
//! nothing; `tests` alone captured its output, for the CI annotations `run_tests`'s own
//! doc comment explains. A run that scrolled past is a run nobody can point at
//! afterward — no file to attach to a report, no per-section timing to say which part
//! got slow, no comparison with yesterday's run.
//!
//! **What this captures a full log for, and what it only times.** A gate section that
//! is one external command with its own argument list (`cargo fmt --check`, `cargo
//! clippy`, `cargo test`, `cargo deny check`) gets its combined stdout/stderr written to
//! `NN-name.log`, exactly what `run_tests` already did for `tests` alone. A section that
//! is itself several pnpm invocations, or an in-process check that prints straight to
//! this process's own stdout (`sync_agents::run`, `report_redaction::check`,
//! `network_isolation::check`, `installer::check_gate`, `frontend::gate_checks`,
//! `scripts::gate_checks`, `ignored_advisories::check`) is only timed and recorded
//! pass/fail in the summary — capturing "everything printed by arbitrary code" would
//! mean redirecting this process's own stdout globally, a change with its own
//! correctness risk for a benefit (a saved copy of output that is already fast and
//! already terse) smaller than the risk. Every section's live console output is
//! therefore unchanged from before this existed; the difference is that the command
//! sections *also* land in a file, and every section's timing lands in the summary.
//!
//! **The trade `run_step` makes.** Capturing a command's output (rather than letting it
//! inherit this process's stdout/stderr and stream live) means the console shows nothing
//! until that command finishes, then shows all of it at once. `run_tests` already made
//! this exact trade for the one section most likely to produce a long, important list of
//! names; this generalizes it to the gate's other command-backed sections. None of them
//! runs long enough — a few seconds each, `clippy` sometimes tens of seconds on a cold
//! cache — for losing line-by-line progress to cost more than a durable log is worth. A
//! byte-correct tee across two OS pipes into one shared file needs real synchronization
//! to avoid interleaving them mid-line; that machinery bought nothing here that capturing
//! does not already give more simply.
//!
//! **Where reports live, and why.** `target/gate-logs/<timestamp>/` — `target/` because
//! this is a build artifact: already `.gitignore`d, already swept by `cargo clean` and
//! the `clean-project` script, and never at risk of landing in the project's own
//! self-export. `latest/` is a plain copy of the most recent run's directory rather than
//! a symlink: creating a symlink on Windows needs `SeCreateSymbolicLinkPrivilege`, a
//! permission most accounts do not hold outside Developer Mode, and a report tool that
//! sometimes fails to write its own "latest" pointer would be worse than one that always
//! copies a few small text files.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use codepack_core::time::UtcDateTime;

/// Runs older than this many gate invocations are deleted, oldest first — a gate run
/// most often exists to diagnose the one that just happened, not to be an archive.
const KEPT_RUNS: usize = 20;

enum SectionStatus {
    Ok,
    Failed,
}

struct SectionRecord {
    name: String,
    status: SectionStatus,
    duration: Duration,
    /// `None` for a timed-only section — see the module doc's capture-vs-time split.
    log_file: Option<String>,
    /// Set only for a failed `tests` section: the names cargo reported under its own
    /// `failures:` block, so the summary names them without a reader opening the log.
    failed_tests: Vec<String>,
}

pub(crate) struct GateReport {
    root: PathBuf,
    dir: PathBuf,
    started_at: UtcDateTime,
    sections: Vec<SectionRecord>,
}

/// A section name as a file-name fragment: `"format (ai-api)"` becomes `"format-ai-api"`,
/// not `"format--ai-api-"` — collapsed and trimmed so a name that already used dashes
/// (most of them do) does not visibly double them up.
fn slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_was_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

impl GateReport {
    pub(crate) fn start(root: &Path) -> Result<Self, String> {
        let started_at = UtcDateTime::now();
        // Colons are not valid in a Windows path component; the audit's own mockup
        // directory name substitutes dashes for exactly this reason.
        let dir_name = started_at.format_iso8601_utc().replace(':', "-");
        let dir = root.join("target/gate-logs").join(&dir_name);
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
        Ok(Self {
            root: root.to_path_buf(),
            dir,
            started_at,
            sections: Vec::new(),
        })
    }

    /// One gate section that is a single external command. Captures combined
    /// stdout/stderr (stdout first, then stderr — matching how `run_tests` already
    /// prints the two), prints it exactly as a live-streamed command would have shown
    /// it overall, writes it to this run's directory, and records the section's outcome
    /// and timing. See the module doc for why this captures rather than streams.
    pub(crate) fn run_step(
        &mut self,
        label: &str,
        program: &str,
        args: &[&str],
    ) -> Result<(), String> {
        println!("\n=== {label} ===");
        println!("$ {program} {}", args.join(" "));
        let started = Instant::now();
        let outcome = Command::new(program)
            .args(args)
            .current_dir(&self.root)
            .output();
        let elapsed = started.elapsed();

        let (ok, combined) = match outcome {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                print!("{stdout}");
                eprint!("{stderr}");
                (output.status.success(), format!("{stdout}{stderr}"))
            }
            Err(error) => {
                let message = format!("failed to launch `{program}`: {error}");
                eprintln!("{message}");
                (false, message)
            }
        };

        self.record(label, ok, elapsed, Some(&combined), Vec::new())?;
        if ok { Ok(()) } else { Err(label.to_string()) }
    }

    /// Times an in-process or multi-command section without capturing its own output —
    /// see the module doc for why. `body` prints to the console exactly as it always
    /// has; this only wraps it with a clock and a result.
    pub(crate) fn timed_section(
        &mut self,
        label: &str,
        body: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        println!("\n=== {label} ===");
        let started = Instant::now();
        let result = body();
        let elapsed = started.elapsed();
        self.record(label, result.is_ok(), elapsed, None, Vec::new())?;
        result
    }

    /// Records the `tests` section from output `run_tests` already captured for its own
    /// CI-annotation purposes — recorded here rather than run a second time, and the
    /// section's own failed-test names ride along so the summary can list them without
    /// a reader opening the log.
    pub(crate) fn record_tests(
        &mut self,
        combined: &str,
        ok: bool,
        elapsed: Duration,
        failed_tests: Vec<String>,
    ) -> Result<(), String> {
        self.record("tests", ok, elapsed, Some(combined), failed_tests.clone())?;
        let junit_path = self
            .dir
            .join(format!("{:02}-tests.junit.xml", self.sections.len().max(1)));
        let junit = junit_xml(combined, elapsed, &failed_tests);
        std::fs::write(&junit_path, junit)
            .map_err(|error| format!("cannot write {}: {error}", junit_path.display()))
    }

    fn record(
        &mut self,
        name: &str,
        ok: bool,
        duration: Duration,
        content: Option<&str>,
        failed_tests: Vec<String>,
    ) -> Result<(), String> {
        let log_file = match content {
            Some(content) => {
                let file_name = format!("{:02}-{}.log", self.sections.len() + 1, slug(name));
                let path = self.dir.join(&file_name);
                std::fs::write(&path, content)
                    .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
                Some(file_name)
            }
            None => None,
        };
        self.sections.push(SectionRecord {
            name: name.to_string(),
            status: if ok {
                SectionStatus::Ok
            } else {
                SectionStatus::Failed
            },
            duration,
            log_file,
            failed_tests,
        });
        Ok(())
    }

    /// Writes `00-summary.txt`, refreshes `target/gate-logs/latest/`, deletes runs past
    /// [`KEPT_RUNS`], and — under CI — appends the same summary to
    /// `$GITHUB_STEP_SUMMARY` so it is visible on the run's own page without a log or an
    /// artifact download.
    ///
    /// Best-effort on every housekeeping step past the summary file itself: a report
    /// tool that fails the whole gate because `latest/` could not be refreshed would be
    /// worse than one that reports the gate's real result and warns about its own
    /// bookkeeping separately.
    pub(crate) fn finish(&self, overall_ok: bool) {
        let summary = self.render_summary(overall_ok);
        let summary_path = self.dir.join("00-summary.txt");
        if let Err(error) = std::fs::write(&summary_path, &summary) {
            eprintln!(
                "gate report: cannot write {}: {error}",
                summary_path.display()
            );
        }
        println!("\n{summary}");
        println!("gate report: {}", self.dir.display());

        if let Err(error) = refresh_latest(&self.root, &self.dir) {
            eprintln!("gate report: could not refresh target/gate-logs/latest: {error}");
        }
        if let Err(error) = rotate_old_runs(&self.root) {
            eprintln!("gate report: could not rotate old runs: {error}");
        }
        if let Ok(step_summary_path) = std::env::var("GITHUB_STEP_SUMMARY")
            && let Err(error) = append_to_file(&step_summary_path, &summary)
        {
            eprintln!("gate report: could not append to $GITHUB_STEP_SUMMARY: {error}");
        }
    }

    fn render_summary(&self, overall_ok: bool) -> String {
        let commit = git_head_commit(&self.root);
        let mut out = format!(
            "gate {}   repository: {commit}\n",
            self.started_at.format_iso8601_utc()
        );
        out.push_str(&"-".repeat(70));
        out.push('\n');

        let mut total = Duration::ZERO;
        for section in &self.sections {
            total += section.duration;
            let status = match section.status {
                SectionStatus::Ok => "ok",
                SectionStatus::Failed => "FAILED",
            };
            out.push_str(&format!(
                "  {:<20} {:<8} {:>7.1}s\n",
                section.name,
                status,
                section.duration.as_secs_f64()
            ));
            // Only for a failed section: a passing one needs no pointer anywhere, and
            // showing it for every section would repeat what the directory listing
            // already says once, at the top.
            if let (SectionStatus::Failed, Some(log_file)) = (&section.status, &section.log_file) {
                out.push_str(&format!("      see {log_file}\n"));
            }
            for test_name in &section.failed_tests {
                out.push_str(&format!("      {test_name}\n"));
            }
        }
        out.push_str(&"-".repeat(70));
        out.push('\n');

        let failed_names: Vec<&str> = self
            .sections
            .iter()
            .filter(|section| matches!(section.status, SectionStatus::Failed))
            .map(|section| section.name.as_str())
            .collect();
        if overall_ok {
            out.push_str(&format!("result: ok, total {:.1}s\n", total.as_secs_f64()));
        } else {
            out.push_str(&format!(
                "result: FAILED ({}), total {:.1}s\n",
                failed_names.join(", "),
                total.as_secs_f64()
            ));
        }
        out
    }
}

fn append_to_file(path: &str, content: &str) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("cannot open {path}: {error}"))?;
    file.write_all(b"```\n")
        .and_then(|_| file.write_all(content.as_bytes()))
        .and_then(|_| file.write_all(b"```\n"))
        .map_err(|error| format!("cannot write {path}: {error}"))
}

/// Copies `run_dir`'s contents into `root/target/gate-logs/latest/`, replacing whatever
/// was there — the plain-copy substitute for a symlink explained in the module doc.
fn refresh_latest(root: &Path, run_dir: &Path) -> Result<(), String> {
    let latest = root.join("target/gate-logs/latest");
    if latest.is_dir() {
        std::fs::remove_dir_all(&latest)
            .map_err(|error| format!("cannot clear {}: {error}", latest.display()))?;
    }
    std::fs::create_dir_all(&latest)
        .map_err(|error| format!("cannot create {}: {error}", latest.display()))?;
    for entry in std::fs::read_dir(run_dir)
        .map_err(|error| format!("cannot list {}: {error}", run_dir.display()))?
        .flatten()
    {
        let path = entry.path();
        if path.is_file() {
            let target = latest.join(entry.file_name());
            std::fs::copy(&path, &target)
                .map_err(|error| format!("cannot copy {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

/// Deletes every timestamped run directory under `target/gate-logs/` past [`KEPT_RUNS`],
/// oldest first by name — the timestamped directory name already sorts chronologically.
fn rotate_old_runs(root: &Path) -> Result<(), String> {
    let gate_logs = root.join("target/gate-logs");
    let mut runs: Vec<PathBuf> = std::fs::read_dir(&gate_logs)
        .map_err(|error| format!("cannot list {}: {error}", gate_logs.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.file_name() != Some("latest".as_ref()))
        .collect();
    runs.sort();
    if runs.len() > KEPT_RUNS {
        for stale in &runs[..runs.len() - KEPT_RUNS] {
            std::fs::remove_dir_all(stale)
                .map_err(|error| format!("cannot remove {}: {error}", stale.display()))?;
        }
    }
    Ok(())
}

fn git_head_commit(root: &Path) -> String {
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|commit| !commit.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .ok()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false);

    if dirty {
        format!("{commit} (dirty)")
    } else {
        commit
    }
}

/// A minimal JUnit XML document for the whole `cargo test --workspace` run: one
/// `<testsuite>`, one `<testcase>` per failed test named by cargo's own `failures:`
/// block. Every workspace test that passed is folded into the suite's own
/// pass/fail counts rather than listed individually — cargo's captured output never
/// names a *passing* test by its full path across every binary, only a per-binary count,
/// so a per-test `<testcase>` for everything that passed would mean re-deriving what
/// cargo already decided not to print.
fn junit_xml(stdout: &str, elapsed: Duration, failed_tests: &[String]) -> String {
    let (total, failed_count) = total_test_counts(stdout);
    // A real total beats a guess, but cargo's own summary lines are only as reliable as
    // its output format staying stable; falling back to what is known for certain — at
    // least the named failures — keeps the count from silently reading as "no tests ran"
    // if that format ever changes underneath this parser.
    let total = total.max(failed_tests.len());
    let failed_count = failed_count.max(failed_tests.len());

    let mut cases = String::new();
    for name in failed_tests {
        cases.push_str(&format!(
            "    <testcase name=\"{}\">\n      <failure/>\n    </testcase>\n",
            xml_escape(name)
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <testsuites>\n\
         \x20 <testsuite name=\"cargo test --workspace\" tests=\"{total}\" \
         failures=\"{failed_count}\" time=\"{:.3}\">\n{cases}\x20 </testsuite>\n\
         </testsuites>\n",
        elapsed.as_secs_f64()
    )
}

/// Sums every binary's own `test result: ok. N passed; M failed; ...` line — cargo
/// prints one per test binary in the workspace, never a workspace-wide total, so the
/// JUnit total has to be added up from all of them by hand.
fn total_test_counts(stdout: &str) -> (usize, usize) {
    let mut passed_total = 0usize;
    let mut failed_total = 0usize;
    for line in stdout.lines() {
        let Some(rest) = line.trim_start().strip_prefix("test result: ") else {
            continue;
        };
        let Some(after_status) = rest.split_once(". ") else {
            continue;
        };
        for field in after_status.1.split(';') {
            let field = field.trim();
            if let Some(count) = field.strip_suffix(" passed") {
                passed_total += count.trim().parse::<usize>().unwrap_or(0);
            } else if let Some(count) = field.strip_suffix(" failed") {
                failed_total += count.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }
    (passed_total + failed_total, failed_total)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_successful_run_summarizes_every_section_with_its_own_timing() {
        let root = tempfile::tempdir().unwrap();
        let mut report = GateReport::start(root.path()).unwrap();
        report
            .record(
                "format",
                true,
                Duration::from_millis(1200),
                None,
                Vec::new(),
            )
            .unwrap();
        report
            .record(
                "clippy",
                true,
                Duration::from_millis(4300),
                None,
                Vec::new(),
            )
            .unwrap();

        let summary = report.render_summary(true);
        assert!(summary.contains("format"));
        assert!(summary.contains("1.2s"));
        assert!(summary.contains("clippy"));
        assert!(summary.contains("4.3s"));
        assert!(summary.contains("result: ok"));
    }

    #[test]
    fn a_failed_section_is_named_in_the_result_line() {
        let root = tempfile::tempdir().unwrap();
        let mut report = GateReport::start(root.path()).unwrap();
        report
            .record("format", true, Duration::from_millis(100), None, Vec::new())
            .unwrap();
        report
            .record(
                "clippy",
                false,
                Duration::from_millis(200),
                None,
                Vec::new(),
            )
            .unwrap();

        let summary = report.render_summary(false);
        assert!(summary.contains("FAILED"));
        assert!(summary.contains("result: FAILED (clippy)"));
    }

    #[test]
    fn failed_test_names_are_listed_under_the_tests_section() {
        let root = tempfile::tempdir().unwrap();
        let mut report = GateReport::start(root.path()).unwrap();
        report
            .record(
                "tests",
                false,
                Duration::from_secs(3),
                Some("cargo output"),
                vec!["module::a_test_that_broke".to_string()],
            )
            .unwrap();

        let summary = report.render_summary(false);
        assert!(summary.contains("module::a_test_that_broke"));
    }

    #[test]
    fn a_section_name_with_punctuation_slugs_without_doubled_dashes() {
        assert_eq!(slug("format (ai-api)"), "format-ai-api");
        assert_eq!(slug("network isolation"), "network-isolation");
    }

    #[test]
    fn a_command_section_writes_its_own_log_file() {
        let root = tempfile::tempdir().unwrap();
        let mut report = GateReport::start(root.path()).unwrap();
        report
            .record(
                "format",
                true,
                Duration::from_millis(1),
                Some("all good"),
                Vec::new(),
            )
            .unwrap();

        let logged = std::fs::read_to_string(report.dir.join("01-format.log")).unwrap();
        assert_eq!(logged, "all good");
    }

    #[test]
    fn a_timed_only_section_writes_no_log_file() {
        let root = tempfile::tempdir().unwrap();
        let mut report = GateReport::start(root.path()).unwrap();
        report
            .record(
                "agents sync",
                true,
                Duration::from_millis(1),
                None,
                Vec::new(),
            )
            .unwrap();

        let entries: Vec<_> = std::fs::read_dir(&report.dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(entries.is_empty(), "{entries:?}");
    }

    #[test]
    fn finish_writes_the_summary_file_and_a_latest_copy() {
        let root = tempfile::tempdir().unwrap();
        let mut report = GateReport::start(root.path()).unwrap();
        report
            .record(
                "format",
                true,
                Duration::from_millis(1),
                Some("ok"),
                Vec::new(),
            )
            .unwrap();

        report.finish(true);

        assert!(report.dir.join("00-summary.txt").is_file());
        let latest = root.path().join("target/gate-logs/latest");
        assert!(latest.join("00-summary.txt").is_file());
        assert!(latest.join("01-format.log").is_file());
    }

    #[test]
    fn a_run_past_the_kept_limit_deletes_the_oldest_first() {
        let root = tempfile::tempdir().unwrap();
        let gate_logs = root.path().join("target/gate-logs");
        std::fs::create_dir_all(&gate_logs).unwrap();
        for index in 0..(KEPT_RUNS + 3) {
            std::fs::create_dir_all(gate_logs.join(format!("2026-01-01T00-00-{index:02}Z")))
                .unwrap();
        }

        rotate_old_runs(root.path()).unwrap();

        let remaining: Vec<String> = std::fs::read_dir(&gate_logs)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(remaining.len(), KEPT_RUNS);
        assert!(!remaining.contains(&"2026-01-01T00-00-00Z".to_string()));
        assert!(remaining.contains(&format!("2026-01-01T00-00-{:02}Z", KEPT_RUNS + 2)));
    }

    #[test]
    fn latest_is_never_counted_or_deleted_as_a_run() {
        let root = tempfile::tempdir().unwrap();
        let gate_logs = root.path().join("target/gate-logs");
        std::fs::create_dir_all(gate_logs.join("latest")).unwrap();
        for index in 0..(KEPT_RUNS + 3) {
            std::fs::create_dir_all(gate_logs.join(format!("2026-01-01T00-00-{index:02}Z")))
                .unwrap();
        }

        rotate_old_runs(root.path()).unwrap();

        assert!(gate_logs.join("latest").is_dir());
    }

    #[test]
    fn junit_xml_names_every_failed_test_as_its_own_case() {
        let xml = junit_xml(
            "test result: FAILED. 8 passed; 2 failed; 0 ignored\n",
            Duration::from_secs(2),
            &["a::b".to_string(), "c::d".to_string()],
        );
        assert!(xml.contains("tests=\"10\""));
        assert!(xml.contains("failures=\"2\""));
        assert!(xml.contains("name=\"a::b\""));
        assert!(xml.contains("name=\"c::d\""));
    }

    #[test]
    fn junit_xml_escapes_a_test_name_that_carries_xml_metacharacters() {
        // Rust identifiers cannot literally carry `<`/`&`, but a generic-parameter or
        // doctest-derived name can render with them, and the writer must not produce
        // invalid XML either way.
        let xml = junit_xml(
            "test result: FAILED. 0 passed; 1 failed; 0 ignored\n",
            Duration::from_secs(1),
            &["a<B> & c".to_string()],
        );
        assert!(xml.contains("a&lt;B&gt; &amp; c"));
        assert!(!xml.contains("a<B>"));
    }

    #[test]
    fn junit_xml_sums_every_binarys_own_result_line_into_one_total() {
        let xml = junit_xml(
            "running 3 tests\n\
             test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n\
             running 5 tests\n\
             test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n",
            Duration::from_secs(4),
            &[],
        );
        assert!(xml.contains("tests=\"8\""), "{xml}");
        assert!(xml.contains("failures=\"0\""), "{xml}");
    }

    #[test]
    fn junit_xml_never_claims_zero_tests_ran_when_named_failures_exist() {
        // If cargo's own summary-line format ever changed underneath the parser, a
        // silent "tests=\"0\"" on a run that actually failed would read as a green,
        // empty suite to a JUnit-consuming dashboard — worse than an undercount.
        let xml = junit_xml(
            "this does not look like cargo's own output at all\n",
            Duration::from_secs(1),
            &["a::known_failure".to_string()],
        );
        assert!(xml.contains("tests=\"1\""), "{xml}");
        assert!(xml.contains("failures=\"1\""), "{xml}");
    }
}
