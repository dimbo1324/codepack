//! Group A report jobs (BLUEPRINT §A.7): the simplest reports, consuming only
//! [`crate::context::Inventory`]/[`crate::context::ReportContext`] — no manifest
//! parsing (Group D), no `codepack_security::ScanResult` (Group B), no `git2` (Group
//! C). Each submodule exposes a `pub const JOB: ReportJob`.

pub mod code_metrics;
pub mod file_statistics;
pub mod large_files;
pub mod summary;
pub mod todo_fixme;

use crate::plugin::ReportJob;

/// The five Group A jobs, in the catalog order from BLUEPRINT §A.7 /
/// `orchestrator.py`'s `report_jobs` list. A future engine stage assembles the full
/// catalog by chaining this with later groups' job lists.
pub fn group_a_jobs() -> [ReportJob; 5] {
    [
        summary::JOB,
        file_statistics::JOB,
        todo_fixme::JOB,
        code_metrics::JOB,
        large_files::JOB,
    ]
}
