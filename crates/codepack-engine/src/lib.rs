//! Orchestrator of the eight-step export pipeline (BLUEPRINT §A.1/§C).
//!
//! ## Scope boundary (this pass, stage S9's Group R+A, `task-checklist.md`)
//!
//! An earlier pass (Group P+C) implemented pipeline steps 1 ("plan") and 2 ("copy"),
//! plus the shared [`paths::build_export_paths`] path derivation every later step
//! needs. This pass adds steps 3 ("structure report": [`structure::write_structure_report`]),
//! 4 ("Git report": [`git_report::write_git_report`]), 5 ("text dump":
//! [`text_dump::write_text_dump`]), 6 ("analytics": [`analytics::run_analytics`]), and 7
//! ("manifest.json + INDEX.md", first write: [`manifest::write_manifest_and_index`]).
//! Step 8 (final archiving), storage close-out (`codepack-storage` wiring), staging
//! cleanup, and the top-level pipeline function that sequences all eight steps arrive in
//! a later pass of the same stage (Group Z).
//!
//! This crate now also depends on `codepack-reports` (step 6's report catalog) and
//! `git2` (step 4's read-only queries) — still never on
//! `codepack-storage`/`codepack-archive`, since Group Z has not landed yet. It never
//! depends on Tauri or the frontend (invariant I8); it never writes into the source
//! project folder (invariant I2); every long-running loop checks the caller-supplied
//! [`codepack_core::CancellationToken`] inside the loop, not only between steps.

mod analytics;
mod copy;
mod error;
mod git_report;
mod ignored_dirs;
mod manifest;
mod paths;
mod plan;
mod relpath;
mod structure;
mod text_dump;
mod timestamp;

pub use analytics::{AnalyticsOutcome, run_analytics};
pub use copy::copy_project;
pub use error::{EngineError, Result};
pub use git_report::write_git_report;
pub use manifest::write_manifest_and_index;
pub use paths::{build_export_paths, sanitize_name};
pub use plan::{PlanOutcome, run_export_plan};
pub use structure::write_structure_report;
pub use text_dump::write_text_dump;
