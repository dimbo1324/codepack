//! [`ReportContext`]: the one struct every report job reads from. Built once per
//! export run by the (future, S9) engine and passed by reference into every
//! [`crate::plugin::ReportJob::run`] call.
//!
//! `scan`/`diff` are `Option` on purpose: this pass (Group G+A) never constructs a
//! real [`codepack_security::ScanResult`] or [`codepack_diff::DiffSelection`] — those
//! only exist once Group B's security-scan adapter and a future diff-aware report
//! actually run the upstream crates — but the fields are real, typed members from day
//! one so a later group's addition of a security/diff-aware report needs no struct
//! redesign (churn the task explicitly asked to avoid).

mod inventory;
mod stack;

pub use inventory::{ExtensionStat, Inventory, InventoryFile, LanguageStat, any_file_name};
pub use stack::{DetectedStack, detect_package_managers, detect_stack};

use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use codepack_core::config::Config;
use codepack_diff::DiffSelection;
use codepack_scanner::ExportPlan;
use codepack_security::ScanResult;

/// Everything a report job needs to read, borrowed from the caller for the duration
/// of one export run.
pub struct ReportContext<'a> {
    pub source_root: PathBuf,
    pub staging_root: PathBuf,
    pub inventory: &'a Inventory,
    pub plan: &'a ExportPlan,
    pub scan: Option<&'a ScanResult>,
    pub diff: Option<&'a DiffSelection>,
    pub config: &'a Config,
    pub cancel: &'a CancellationToken,
    pub profile: &'a str,
}

impl<'a> ReportContext<'a> {
    /// [`crate::paths::resolve`] against [`Self::staging_root`] — the one place a
    /// report should turn a `PlannedFile.relative_path` back into an openable file.
    pub fn resolve(&self, relative_path: &str) -> PathBuf {
        crate::paths::resolve(&self.staging_root, relative_path)
    }
}

/// Redacts secrets from a single line of report output before it is written.
/// Every report that quotes raw file content (as opposed to just a file path or a
/// computed statistic) must route the quoted text through this — invariant I3
/// (`docs/architecture/invariants.md`): a finding's/line's text is redacted before it
/// reaches a report.
pub fn redact_line(line: &str) -> String {
    codepack_security::redact_secrets(line)
}

/// True when `staging_root`/`name` exists on disk. A single, bounded existence check
/// — not a directory walk (this crate's scope boundary).
pub fn root_entry_exists(root: &Path, name: &str) -> bool {
    root.join(name).exists()
}
