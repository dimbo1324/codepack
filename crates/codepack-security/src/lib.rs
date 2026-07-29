//! Export safety modes, secret redaction, and the heuristic secret detector.
//!
//! Stage S3 scope boundary (binding, see `docs/__arch__/ROADMAP.md` and
//! `.ai/project/12-domain-rules.md`): this crate depends only on `codepack-core`. It
//! takes a caller-supplied file list — it never walks the filesystem in production
//! code (that is `codepack-scanner`'s job; combining the two crates is S9's). It never
//! touches SQLite, the clipboard, or any text-dump call site. It never performs
//! network access or network validation of a secret (invariant I1, permanent):
//! every detector here is a pure, local, in-memory pattern match.

pub mod classify;
mod constants;
pub mod error;
pub mod patterns;
pub mod policy;
pub mod redact;
pub mod scan;

pub use constants::{
    BALANCED_MODE_EXCLUDED_SUFFIXES, HIGH_RISK_FILENAMES, SAFE_EXPORT_MODES,
    SAFE_MODE_EXCLUDED_SUFFIXES, SENSITIVE_FILENAMES, SENSITIVE_SUFFIXES,
};
pub use error::{Result, SecurityError};
pub use policy::{
    SafetyDecision, SecurityOptions, classify_sensitive_file, is_env_example, normalise_mode,
    should_skip_file_for_safety,
};
pub use redact::redact_secrets;
pub use scan::{Finding, FindingKind, ScanResult, ScanSummary, scan_project};
