//! Stage S13's API path: ask a provider a question about a bundle.
//!
//! **The one crate in this workspace that may reach the network** — invariant I1's single
//! named exception, and the only place `ureq` and `keyring` are declared. The
//! `network isolation` step of `cargo xtask gate` fails if any other crate declares a
//! client, and fails if anything but `codepack-cli` or `codepack-desktop` depends on this
//! one: a transport reachable from the engine would sit underneath every export, where no
//! user action gates it.
//!
//! ## How it got here
//!
//! It used to be the `api` feature of `codepack-ai`, on by default. Both front ends took
//! that crate with `default-features = false`, so no binary linked a transport — but
//! `codepack-ai` is a *workspace member*, and a member is compiled with its own defaults
//! by `cargo test --workspace`. So `keyring` and `ureq` were built on every platform, for
//! code no user could reach. On Linux `keyring` wants a Secret Service backend, which
//! made a dead code path into a live obstacle to building on anything but Windows
//! (audit 2026-09-05 No. 26; owner decision 2026-09-06, Q41). The feature became this
//! crate, and the crate was excluded from the workspace.
//!
//! Being excluded also meant not being built by the gate — preserved, not maintained, as
//! this comment used to say. Owner decision 2026-09-12 resolved that the other way:
//! rather than keep a dead path alive, finish the stage. It now has a command
//! (`codepack ask`, `codepack key`) and a screen, so the crate is back in the workspace,
//! and the build cost the exclusion avoided is a cost the feature buys something with.
//!
//! ## What the exception is worth
//!
//! Nothing here runs on its own. A send happens because a user asked for one, on a bundle
//! they already exported, after [`plan::SendPlan::check`] has agreed — and that guard runs
//! *before* the key is read, so a refused send is not even observed by the credential
//! store, let alone by a provider.

pub mod keys;
pub mod plan;
pub mod provider;
pub mod providers;

use std::path::Path;

pub use codepack_ai::{AiError, Refusal};
pub use plan::SendPlan;
pub use provider::{AiAnswer, AiProvider, AiRequest, ModelInfo};
pub use providers::DEFAULT_PROVIDER;

/// The whole API path, in the order it must happen.
///
/// A single entry point rather than four exported steps, because the steps are not
/// independent: the guard has to run after the plan is built and before the key is read,
/// and a caller free to reorder them is a caller free to send an unchecked bundle.
///
/// `override_critical` is threaded from an explicit user action; see
/// [`plan::SendPlan::check`] for why it exists and why it is not a default.
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

#[cfg(test)]
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
