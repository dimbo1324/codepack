//! Orchestrator of the eight-step export pipeline (BLUEPRINT §A.1/§C).
//!
//! ## Scope boundary (this pass, stage S9's Group P+C, `task-checklist.md`)
//!
//! This pass implements only pipeline steps 1 ("plan": [`plan::run_export_plan`]) and
//! 2 ("copy": [`copy::copy_project`]), plus the shared [`paths::build_export_paths`]
//! path derivation every later step needs. Steps 3-8 (structure/Git/text-dump
//! reports, the analytics/manifest pass, final archiving, and storage close-out) and
//! the top-level pipeline function that sequences all eight steps arrive in later
//! passes of the same stage.
//!
//! This crate depends on `codepack-core`, `codepack-scanner`, `codepack-security`, and
//! `codepack-diff` — never on `codepack-storage`/`codepack-archive`/`codepack-reports`
//! yet, since this pass does not touch the steps that need them. It never depends on
//! Tauri or the frontend (invariant I8); it never writes into the source project
//! folder (invariant I2); every long-running loop checks the caller-supplied
//! [`codepack_core::CancellationToken`] inside the loop, not only between steps.

mod copy;
mod error;
mod ignored_dirs;
mod paths;
mod plan;
mod timestamp;

pub use copy::copy_project;
pub use error::{EngineError, Result};
pub use paths::{build_export_paths, sanitize_name};
pub use plan::{PlanOutcome, run_export_plan};
