//! Group A report jobs (BLUEPRINT §A.7): the simplest reports, consuming only
//! [`crate::context::Inventory`]/[`crate::context::ReportContext`] — no manifest
//! parsing (Group D), no `codepack_security::ScanResult` (Group B), no `git2` (Group
//! C). Each submodule exposes a `pub const JOB: ReportJob`.
//!
//! Group B ([`security_scan`]) and Group D ([`dependencies`], [`scripts`], [`config`],
//! [`docker`], [`dependency_intelligence`]) were added by an earlier pass over the same
//! shared glue. This pass adds Group C ([`git_deep`], [`git_timeline`] — read-only
//! `git2` queries) and Group E: the heuristic/derived reports that synthesize
//! Inventory/manifest/graph data ([`routes_and_pages`], [`dependency_graph`],
//! [`architecture`], [`key_files`], [`code_quality`], [`api_surface`], [`frontend`],
//! [`backend`], [`project_health`], [`refactoring`], [`architecture_map`], plus the
//! shared [`layout`] directory helper). Their job lists are exposed the same way, via
//! [`group_c_jobs`] and [`group_e_jobs`].

pub mod api_surface;
pub mod architecture;
pub mod architecture_map;
pub mod backend;
pub mod code_metrics;
pub mod code_quality;
pub mod config;
pub mod dependencies;
pub mod dependency_graph;
pub mod dependency_intelligence;
pub mod docker;
pub mod file_statistics;
pub mod frontend;
pub mod git_deep;
pub mod git_timeline;
pub mod key_files;
pub mod large_files;
mod layout;
pub mod project_health;
pub mod refactoring;
pub mod routes_and_pages;
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

/// Group C: the read-only `git2` reports.
pub fn group_c_jobs() -> [ReportJob; 2] {
    [git_deep::JOB, git_timeline::JOB]
}

/// Group E: the heuristic/derived reports, in BLUEPRINT §A.7 catalog order.
pub fn group_e_jobs() -> [ReportJob; 11] {
    [
        routes_and_pages::JOB,
        dependency_graph::JOB,
        architecture::JOB,
        key_files::JOB,
        code_quality::JOB,
        api_surface::JOB,
        frontend::JOB,
        backend::JOB,
        project_health::JOB,
        refactoring::JOB,
        architecture_map::JOB,
    ]
}
