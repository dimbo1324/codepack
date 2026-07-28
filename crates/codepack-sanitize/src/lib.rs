//! "Sterile copy": a standalone action, not a step of `codepack-engine`'s 8-step
//! pipeline. Takes a source project folder and produces a **separate destination
//! folder** with comments stripped via real tree-sitter parsers (never regex) and,
//! where an external formatter is found on `PATH`, reformatted by it.
//!
//! Scope (`docs/decisions/open-questions.md`, 2026-07-28 — "Стерильная копия"): this
//! crate depends only on `codepack-core`, `codepack-scanner`, `codepack-security`.
//! `codepack-engine` neither calls it nor is called by it, and it never touches the
//! ZIP/archive contract or any of the ~30 existing report artifacts — its own artifact
//! is `STERILE_COPY_REPORT.md`/`.json` (`report.rs`), versioned with `schema_version`
//! from its first release since it is a brand-new contract, not an existing one
//! invariant I5 already governs.
//!
//! File selection reuses `codepack_scanner::build_export_plan` (base/stack/custom-rule
//! filtering and text-vs-binary classification); safety filtering reuses
//! `codepack_security::should_skip_file_for_safety`; secret redaction reuses
//! `codepack_security::redact_secrets` — never a parallel reimplementation of any of the
//! three, so a sterile copy can never become a second, less-guarded way for a secret to
//! leave the project (invariant I3).

mod error;
mod language;
mod options;
mod report;
mod run;
mod strip;

pub use error::{Result, SanitizeError};
pub use language::{Language, detect_language};
pub use options::{FileOutcome, SterileCopyOptions, SterileCopyReport, SterileCopySummary};
pub use report::SCHEMA_VERSION;
pub use run::run_sterile_copy;
