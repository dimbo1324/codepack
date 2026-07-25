//! Tree walking, ignore rules, stack detection, and export planning.
//!
//! ## Scope boundary (stage S2, `ROADMAP.md` §2)
//!
//! [`build_export_plan`] applies **only** base-ignore pruning (`IGNORED_DIR_NAMES`),
//! stack-detected extra ignore directories ([`detect_stacks`]), and
//! `.exportignore`/custom-rule filtering ([`ExportIgnoreRules`]). It deliberately does
//! **not** apply:
//!
//! - safe-export-mode filtering (secrets/high-risk-file exclusion) — that is
//!   `codepack-security`, stage S3;
//! - diff or incremental selection (only-changed-files filtering) — that is
//!   `codepack-diff`, stage S4.
//!
//! Every file this stage's plan marks `"excluded"` was excluded by a base/stack/custom
//! rule; every file it marks `"included"` would still need to pass S3's and S4's
//! filters once those stages exist. `ExportPlan.severity` therefore never reaches
//! `"high"`/`"critical"` within this stage's own logic — those values are reserved for
//! S3's safety reasons.
//!
//! The source directory this crate walks is never written to (invariant I2); symlinks
//! are never followed while walking (invariant I7).

mod classify;
mod constants;
mod error;
pub mod ignore;
pub mod plan;
mod stack;
mod walk;

pub use classify::{BINARY_SAMPLE_BYTES, looks_binary, should_consider_text_file};
pub use constants::{
    BINARY_EXTENSIONS, IGNORED_DIR_NAMES, TEXT_EXTENSIONS, TEXT_FILENAMES_WITHOUT_EXTENSION,
};
pub use error::{Result, ScannerError};
pub use ignore::{ExportIgnoreRules, RulesReport, ScanOptions};
pub use plan::{
    ExportPlan, PlannedFile, SafetyClassifier, build_export_plan, no_safety_classification,
    write_export_plan_files,
};
pub use stack::{StackInfo, detect_stacks, merged_extra_ignored_dirs, primary_stack};
pub use walk::{IgnoredDirMatcher, SkippedDir, WalkOutcome, WalkedFile, walk_project};
