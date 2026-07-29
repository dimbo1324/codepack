//! Byte and token accounting, and context-budget selection
//! (BLUEPRINT §B.2/§B.3/§E.1/§E.5).
//!
//! ## Scope boundary (stage S6, `docs/__arch__/ROADMAP.md`)
//!
//! This crate has no dependency on any other `codepack-*` crate — it is a pure,
//! config-agnostic computation library. It never computes a per-file "importance"
//! score: [`fit_to_budget`] takes `importance` as a caller-supplied value on
//! [`BudgetCandidate`], and a future `codepack-reports` job
//! (`16_key_files_report`/`22_project_health_report`) is the one that computes it.
//!
//! No network access, ever: [`ModelContextLimits::default`] is a bundled, hardcoded
//! table (verbatim from the archived legacy implementation's `MODEL_CONTEXT_LIMITS`
//! dict), never fetched remotely. Loading an override file or resolving a config path
//! to extend this table is a documented future extension point, not something
//! implemented in this stage.
//!
//! Both [`estimate_tokens_fallback`] and [`estimate_tokens_refined`] stay public and
//! independently callable (invariant I4, `docs/architecture/invariants.md`): the
//! refined, content-aware estimate never silently replaces the legacy flat-ratio
//! fallback anywhere in this crate's API. Byte-size reporting ([`format_bytes`])
//! likewise stays independent of both — tokens are additive, never a replacement.

mod budget;
mod bytes;
mod model_limits;
mod tokens;

pub use budget::{BudgetCandidate, BudgetSelection, ExclusionReason, fit_to_budget};
pub use bytes::format_bytes;
pub use model_limits::ModelContextLimits;
pub use tokens::{TokenCoefficients, estimate_tokens_fallback, estimate_tokens_refined};
