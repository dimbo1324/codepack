//! Group A report jobs (BLUEPRINT §A.7): the simplest reports, consuming only
//! [`crate::context::Inventory`]/[`crate::context::ReportContext`] — no manifest
//! parsing (Group D), no `codepack_security::ScanResult` (Group B), no `git2` (Group
//! C). Each submodule exposes a `pub const JOB: ReportJob`.
//!
//! Group B ([`security_scan`]) and Group D ([`dependencies`], [`scripts`], [`config`],
//! [`docker`], [`dependency_intelligence`]) are added by a later pass over the same
//! shared glue; their job lists are exposed the same way, via [`group_b_jobs`] and
//! [`group_d_jobs`].

pub mod code_metrics;
pub mod config;
pub mod dependencies;
pub mod dependency_intelligence;
pub mod docker;
pub mod file_statistics;
pub mod large_files;
pub mod scripts;
pub mod security_scan;
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

/// Group B: the security-scan adapter (`06_security_scan.{txt,json,sarif}`).
pub fn group_b_jobs() -> [ReportJob; 1] {
    [security_scan::JOB]
}

/// Group D: the manifest-parser reports, in BLUEPRINT §A.7 catalog order.
pub fn group_d_jobs() -> [ReportJob; 5] {
    [
        dependencies::JOB,
        scripts::JOB,
        config::JOB,
        docker::JOB,
        dependency_intelligence::JOB,
    ]
}
