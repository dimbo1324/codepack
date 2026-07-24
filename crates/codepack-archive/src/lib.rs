//! ZIP building, logical splitting, and restore helpers (BLUEPRINT §A.8).
//!
//! ## Scope boundary (stage S8, `ROADMAP.md`)
//!
//! This crate depends only on `codepack-core`. It does not decide which files belong
//! in the export — that already happened upstream (`codepack-scanner`'s planning,
//! `codepack-security`'s safety modes, `codepack-diff`'s selection); this crate takes
//! an already-staged directory tree (`codepack_core::ExportPaths::staging_dir`) and
//! archives it, nothing more.
//!
//! Legacy fires two hooks around archiving: a hardcoded dashboard-refresh call and a
//! generic `pre_archive_hook` callback. Both are collapsed here into a single
//! caller-supplied `on_plan_ready` callback (see [`build_final_archives`]) — not a
//! `codepack-archive` → `codepack-reports` dependency, which would violate the
//! downward-only dependency rule (`.ai/project/12-domain-rules.md`).

mod build;
mod entry;
mod error;
mod options;
mod plan;
mod report;
mod restore;
mod timestamp;

pub use build::build_final_archives;
pub use entry::{ArchiveEntry, classify_archive_group, collect_entries};
pub use error::{ArchiveError, Result};
pub use options::ArchiveOptions;
pub use plan::{ArchivePartPlan, ArchivePlan, plan_archive, predicted_result_for_plan};
pub use report::write_archive_plan_report;
pub use restore::{extract_zip_safely, restore_archive_set, safe_member_target};
