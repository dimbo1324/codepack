//! Stage S13 — direct AI integration.
//!
//! **This is the only crate in the workspace permitted to reach the network.** Invariant
//! I1 says all analysis is local and no crate touches the network; S13 is its single,
//! named exception, and only on an explicit user action. `cargo xtask gate` fails the
//! build if any other crate declares an HTTP client, so the exception is enforced by the
//! build rather than by everyone remembering the rule.
//!
//! Two paths, and they solve different problems:
//!
//! * [`plan`] + [`providers`] — the API path (BLUEPRINT §B.8). The key lives in the OS
//!   credential store ([`keys`]), the user confirms a [`plan::SendPlan`] first, and the
//!   answer is written back into the bundle.
//! * [`handoff`] — the offline path. Claude Code and Codex already read the filesystem,
//!   so they get a prompt file next to the bundle and a command to run. No network, no
//!   key, invariant I1 untouched.
//!
//! Nothing here starts on its own. Every entry point is called from a user action, and
//! the guards in [`plan::SendPlan::check`] run before anything leaves the machine.
//!
//! ## The `api` feature
//!
//! Only the API path is behind the `api` feature (on by default); [`handoff`] and
//! [`error`] are always compiled. A dependent that offers the offline path alone —
//! which is both front ends today — takes this crate with `default-features = false`
//! and never links an HTTP client or the credential store at all. That is invariant I1
//! made structural instead of trusted: `cargo xtask gate` can only read declared
//! dependencies, so "the binary contains a transport it never calls" is exactly the
//! shape of risk it cannot see, and the fix is not to have the transport.

pub mod error;
pub mod handoff;
#[cfg(feature = "api")]
pub mod keys;
#[cfg(feature = "api")]
pub mod plan;
#[cfg(feature = "api")]
pub mod provider;
#[cfg(feature = "api")]
pub mod providers;

pub use error::{AiError, Refusal};
pub use handoff::{AGENTS, Handoff, LocalAgent};
#[cfg(feature = "api")]
pub use plan::SendPlan;
#[cfg(feature = "api")]
pub use provider::{AiAnswer, AiProvider, AiRequest, ModelInfo};
#[cfg(feature = "api")]
pub use providers::DEFAULT_PROVIDER;

#[cfg(feature = "api")]
use std::path::Path;

/// The whole API path, in the order it must happen.
///
/// A single entry point rather than four exported steps, because the steps are not
/// independent: the guard has to run after the plan is built and before the key is read,
/// and a caller free to reorder them is a caller free to send an unchecked bundle.
///
/// `override_critical` is threaded from an explicit user action; see
/// [`plan::SendPlan::check`] for why it exists and why it is not a default.
#[cfg(feature = "api")]
pub fn ask(
    bundle_dir: &Path,
    provider_id: &str,
    model: &str,
    question: &str,
    enabled: bool,
    override_critical: bool,
) -> Result<AiAnswer, AiError> {
    let provider = providers::resolve(provider_id)?;
    let plan = plan::build_plan(bundle_dir, provider.as_ref(), model)?;
    plan.check(enabled, override_critical)?;

    let request = plan::build_request(bundle_dir, model, question)?;
    let key = keys::load_key(provider_id)?;
    let answer = provider.ask(&key, &request)?;

    // Best effort: the answer is already in hand, and failing to file it away must not
    // turn a successful exchange into an error the user reads as "it did not work".
    let _ = plan::save_answer(bundle_dir, question, &answer.text);
    Ok(answer)
}

#[cfg(all(test, feature = "api"))]
mod tests {
    use super::*;

    #[test]
    fn a_disabled_integration_never_reaches_the_key_store() {
        // The ordering matters: `check` runs before `load_key`, so a disabled or refused
        // send cannot even be observed by the credential store, let alone the network.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("AI_CONTEXT")).unwrap();
        std::fs::write(dir.path().join("AI_CONTEXT").join("00.md"), "x").unwrap();

        let error = ask(
            dir.path(),
            DEFAULT_PROVIDER,
            "claude-opus-5",
            "q",
            false,
            false,
        )
        .unwrap_err();
        assert!(matches!(error, AiError::Refused(Refusal::Disabled)));
    }

    #[test]
    fn an_unknown_provider_fails_before_any_bundle_is_read() {
        let error = ask(Path::new("does-not-exist"), "nope", "m", "q", true, false).unwrap_err();
        assert!(matches!(error, AiError::UnknownProvider { .. }));
    }

    #[test]
    fn a_critical_finding_stops_the_send_before_the_key_is_read() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("AI_CONTEXT")).unwrap();
        std::fs::write(dir.path().join("AI_CONTEXT").join("00.md"), "x").unwrap();
        std::fs::write(
            dir.path().join("06_security_scan.json"),
            r#"{"findings":[{"severity":"critical"}]}"#,
        )
        .unwrap();

        let error = ask(
            dir.path(),
            DEFAULT_PROVIDER,
            "claude-opus-5",
            "q",
            true,
            false,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            AiError::Refused(Refusal::CriticalFindings { count: 1 })
        ));
    }
}
