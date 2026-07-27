//! The staging-directory cleanup guard, and its tests.
//!
//! Split out of `orchestrator.rs` on 2026-07-27 (finding 6, audit): that file was
//! 688 lines, past the project's own ~600-line limit. This is a self-contained
//! concern - one RAII type with one job, plus the tests that were already only
//! about it - so it moves whole rather than being sliced to hit a number.

use std::path::PathBuf;

use codepack_core::{LogEvent, LogLevel, ProgressEvent, ProgressSender};

/// RAII guard: removes the staging directory on every exit path from `run_export`
/// (success, an early `?` error return, or an unwinding panic) unless `keep` is set --
/// never only on the happy path. A plain function-tail `remove_dir_all` call would be
/// skipped by any of this function's many `?` early returns, silently leaking the
/// staging directory on genuine I/O/storage failures (a real gap this pass's own review
/// found: the task-checklist's claim of "unconditional... on every code path" was true
/// for cancellation but not for hard errors).
pub(super) struct StagingCleanupGuard<'a> {
    pub(super) staging_dir: PathBuf,
    pub(super) keep: bool,
    pub(super) progress: &'a ProgressSender,
}

impl Drop for StagingCleanupGuard<'_> {
    fn drop(&mut self) {
        if self.keep || !self.staging_dir.exists() {
            return;
        }
        if let Err(err) = std::fs::remove_dir_all(&self.staging_dir) {
            let _ = self.progress.send(ProgressEvent::Log(LogEvent {
                level: LogLevel::Info,
                message: format!("failed to remove staging directory: {err}"),
            }));
        }
    }
}

#[cfg(test)]
mod staging_cleanup_guard_tests {
    use super::StagingCleanupGuard;

    #[test]
    fn removes_the_staging_directory_when_dropped_and_not_kept() {
        let dir = tempfile::tempdir().unwrap();
        let staging_dir = dir.path().join("staging");
        std::fs::create_dir_all(&staging_dir).unwrap();
        assert!(staging_dir.exists());

        let (progress, _rx) = codepack_core::progress_channel();
        {
            let _guard = StagingCleanupGuard {
                staging_dir: staging_dir.clone(),
                keep: false,
                progress: &progress,
            };
        }

        assert!(
            !staging_dir.exists(),
            "the guard must remove the staging directory on drop"
        );
    }

    #[test]
    fn leaves_the_staging_directory_in_place_when_keep_is_set() {
        let dir = tempfile::tempdir().unwrap();
        let staging_dir = dir.path().join("staging");
        std::fs::create_dir_all(&staging_dir).unwrap();

        let (progress, _rx) = codepack_core::progress_channel();
        {
            let _guard = StagingCleanupGuard {
                staging_dir: staging_dir.clone(),
                keep: true,
                progress: &progress,
            };
        }

        assert!(
            staging_dir.exists(),
            "keep_staging_folder must be honored by the guard, not just the happy path"
        );
    }

    #[test]
    fn runs_on_an_early_return_via_the_question_mark_operator_not_only_on_success() {
        fn fails_after_constructing_the_guard(
            staging_dir: &std::path::Path,
            progress: &codepack_core::ProgressSender,
        ) -> Result<(), std::io::Error> {
            let _guard = StagingCleanupGuard {
                staging_dir: staging_dir.to_path_buf(),
                keep: false,
                progress,
            };
            Err(std::io::Error::other("a genuine mid-pipeline failure"))?;
            Ok(())
        }

        let dir = tempfile::tempdir().unwrap();
        let staging_dir = dir.path().join("staging");
        std::fs::create_dir_all(&staging_dir).unwrap();

        let (progress, _rx) = codepack_core::progress_channel();
        let result = fails_after_constructing_the_guard(&staging_dir, &progress);

        assert!(result.is_err());
        assert!(
            !staging_dir.exists(),
            "an early `?` error return must still trigger staging cleanup, matching the \
             task-checklist's own 'unconditional on every code path' claim"
        );
    }
}
