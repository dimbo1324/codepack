//! [`ReportJob`]/[`run_reports`]: the flat plugin table and fault-tolerant runner,
//! ported from legacy `reports/insights/plugins.py`/`orchestrator.py`'s per-plugin
//! loop. A job's own `run` returning `Err`, *or* genuinely panicking, is caught here
//! and turned into an `ERROR_<name>.txt` artifact — the rest of the run continues
//! either way (BLUEPRINT §A.7's fault-tolerance rule; unlike legacy's exception-blind
//! `except Exception`, a Rust `panic!` does not itself unwind past a Python-style
//! `try`, so [`std::panic::catch_unwind`] is required to reach parity).

use std::fs;
use std::path::Path;

use crate::context::ReportContext;
use crate::error::ReportError;
use crate::profile;

pub struct ReportJob {
    pub filename: &'static str,
    pub profiles: &'static [&'static str],
    pub description: &'static str,
    pub run: fn(&ReportContext<'_>, &Path) -> Result<(), ReportError>,
}

impl ReportJob {
    pub fn should_run(&self, profile: &str) -> bool {
        profile::should_run(profile, self.profiles)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RunSummary {
    pub succeeded: Vec<&'static str>,
    pub failed: Vec<(&'static str, String)>,
    pub skipped: Vec<&'static str>,
    pub cancelled: bool,
    /// Set when a `ERROR_<name>.txt` diagnostic itself could not be written — best
    /// effort, never causes the run to stop early.
    pub diagnostic_write_failures: Vec<&'static str>,
}

/// Runs every job in `jobs`, in order, against `ctx`, writing each job's output under
/// `out_dir`. Skips a job whose profile set does not match `ctx.profile`; stops (but
/// does not fail) the remaining jobs once cancellation is observed between jobs.
/// A single job's failure — `Err` or panic — never aborts the run.
pub fn run_reports(jobs: &[ReportJob], ctx: &ReportContext<'_>, out_dir: &Path) -> RunSummary {
    let mut summary = RunSummary::default();

    for job in jobs {
        if ctx.cancel.is_cancelled() {
            summary.cancelled = true;
            break;
        }
        if !job.should_run(ctx.profile) {
            summary.skipped.push(job.filename);
            continue;
        }

        let output_file = out_dir.join(job.filename);
        let run = job.run;
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(ctx, &output_file)));

        match outcome {
            Ok(Ok(())) => summary.succeeded.push(job.filename),
            Ok(Err(err)) => {
                record_failure(&mut summary, out_dir, job.filename, err.to_string());
            }
            Err(payload) => {
                record_failure(
                    &mut summary,
                    out_dir,
                    job.filename,
                    panic_message(payload.as_ref()),
                );
            }
        }
    }

    summary
}

fn record_failure(
    summary: &mut RunSummary,
    out_dir: &Path,
    filename: &'static str,
    message: String,
) {
    if write_error_file(out_dir, filename, &message).is_err() {
        summary.diagnostic_write_failures.push(filename);
    }
    summary.failed.push((filename, message));
}

/// Legacy: `safe_filename = filename.replace("/", "_").replace("\\", "_")`, then
/// `ERROR_{safe_filename}.txt` — note this deliberately does **not** strip the
/// original extension, so e.g. `01_summary.txt` yields `ERROR_01_summary.txt.txt`.
fn write_error_file(out_dir: &Path, filename: &str, message: &str) -> std::io::Result<()> {
    let sanitized = filename.replace(['/', '\\'], "_");
    let error_path = out_dir.join(format!("ERROR_{sanitized}.txt"));
    let body = format!("Failed to create {filename}\n\n{message}\n");
    fs::write(error_path, body)
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "report job panicked with a non-string payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{Inventory, ReportContext};
    use codepack_core::CancellationToken;
    use codepack_core::config::Config;
    use codepack_scanner::{ExportIgnoreRules, ScanOptions, build_export_plan};

    fn fixture_ctx(
        dir: &std::path::Path,
    ) -> (codepack_scanner::ExportPlan, Config, CancellationToken) {
        let plan = build_export_plan(
            dir,
            &ScanOptions::default(),
            &ExportIgnoreRules::default(),
            &codepack_scanner::no_safety_classification,
            &CancellationToken::new(),
        )
        .unwrap();
        (plan, Config::default(), CancellationToken::new())
    }

    fn ok_job(filename: &'static str, profiles: &'static [&'static str]) -> ReportJob {
        ReportJob {
            filename,
            profiles,
            description: "test job",
            run: |_ctx, output| {
                fs::write(output, "ok\n").map_err(|source| ReportError::Write {
                    path: output.to_path_buf(),
                    source,
                })
            },
        }
    }

    fn failing_job(filename: &'static str) -> ReportJob {
        ReportJob {
            filename,
            profiles: profile::ALL_PROFILES,
            description: "always fails",
            run: |_ctx, _output| {
                Err(ReportError::Write {
                    path: std::path::PathBuf::from("nope"),
                    source: std::io::Error::other("boom"),
                })
            },
        }
    }

    fn panicking_job(filename: &'static str) -> ReportJob {
        ReportJob {
            filename,
            profiles: profile::ALL_PROFILES,
            description: "always panics",
            run: |_ctx, _output| panic!("deliberate test panic"),
        }
    }

    #[test]
    fn failing_job_writes_error_file_and_others_still_complete() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.py"), "x = 1\n").unwrap();
        let (plan, config, cancel) = fixture_ctx(dir.path());
        let inventory = Inventory::from_plan(&plan);
        let ctx = ReportContext {
            source_root: dir.path().to_path_buf(),
            staging_root: dir.path().to_path_buf(),
            inventory: &inventory,
            plan: &plan,
            scan: None,
            diff: None,
            config: &config,
            cancel: &cancel,
            profile: "full",
        };
        let out_dir = tempfile::tempdir().unwrap();

        let jobs = vec![
            failing_job("01_fails.txt"),
            panicking_job("02_panics.txt"),
            ok_job("03_ok.txt", profile::ALL_PROFILES),
        ];
        let summary = run_reports(&jobs, &ctx, out_dir.path());

        assert_eq!(summary.succeeded, vec!["03_ok.txt"]);
        assert_eq!(summary.failed.len(), 2);
        assert!(!summary.cancelled);

        let error_one = out_dir.path().join("ERROR_01_fails.txt.txt");
        assert!(error_one.exists());
        let content_one = std::fs::read_to_string(&error_one).unwrap();
        assert!(content_one.contains("01_fails.txt"));
        assert!(content_one.contains("boom"));

        let error_two = out_dir.path().join("ERROR_02_panics.txt.txt");
        assert!(error_two.exists());
        let content_two = std::fs::read_to_string(&error_two).unwrap();
        assert!(content_two.contains("deliberate test panic"));

        assert!(out_dir.path().join("03_ok.txt").exists());
    }

    #[test]
    fn profile_gating_skips_unlisted_jobs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.py"), "x = 1\n").unwrap();
        let (plan, config, cancel) = fixture_ctx(dir.path());
        let inventory = Inventory::from_plan(&plan);
        let ctx = ReportContext {
            source_root: dir.path().to_path_buf(),
            staging_root: dir.path().to_path_buf(),
            inventory: &inventory,
            plan: &plan,
            scan: None,
            diff: None,
            config: &config,
            cancel: &cancel,
            profile: "minimal",
        };
        let out_dir = tempfile::tempdir().unwrap();

        let jobs = vec![
            ok_job("quick_only.txt", &["quick"]),
            ok_job("minimal_ok.txt", &["minimal"]),
        ];
        let summary = run_reports(&jobs, &ctx, out_dir.path());

        assert_eq!(summary.skipped, vec!["quick_only.txt"]);
        assert_eq!(summary.succeeded, vec!["minimal_ok.txt"]);
    }

    #[test]
    fn cancellation_stops_remaining_jobs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.py"), "x = 1\n").unwrap();
        let (plan, config, cancel) = fixture_ctx(dir.path());
        cancel.cancel();
        let inventory = Inventory::from_plan(&plan);
        let ctx = ReportContext {
            source_root: dir.path().to_path_buf(),
            staging_root: dir.path().to_path_buf(),
            inventory: &inventory,
            plan: &plan,
            scan: None,
            diff: None,
            config: &config,
            cancel: &cancel,
            profile: "full",
        };
        let out_dir = tempfile::tempdir().unwrap();

        let jobs = vec![ok_job("never_runs.txt", profile::ALL_PROFILES)];
        let summary = run_reports(&jobs, &ctx, out_dir.path());

        assert!(summary.cancelled);
        assert!(summary.succeeded.is_empty());
        assert!(!out_dir.path().join("never_runs.txt").exists());
    }
}
