//! Stage S13, both halves: handing a bundle to a local coding agent, and asking a
//! provider about one over the network.
//!
//! ## The offline half
//!
//! Nothing is launched. `prepare_handoff` writes `AI_HANDOFF.md` into the bundle and
//! returns the command to run there; the user starts their own agent, in their own
//! terminal, where they can see what it does. Spawning a process would also need a
//! capability the webview deliberately does not have.
//!
//! ## The API half, and what holds invariant I1 here
//!
//! Until 2026-09-12 this module said the desktop binary linked no HTTP client at all.
//! It does now — `codepack-ai-api` is one of two crates permitted to carry one, and this
//! is one of two permitted to depend on it — so the invariant rests on the guard rather
//! than on the absence of a dependency. That guard is
//! [`codepack_ai_api::plan::SendPlan::check`], and it runs before the key is read, so a
//! refused send is not observed by the credential store, let alone by a provider. Three
//! things gate a send from this window:
//!
//! 1. `Config::ai_api_enabled`, off in a fresh installation.
//! 2. A [`crate::commands::ValidatedResultPath`] — the bundle has to be an export this
//!    installation actually produced, the same boundary every other bundle command
//!    crosses.
//! 3. The critical-findings refusal, overridable only by an argument the frontend sets
//!    from a deliberate second action.
//!
//! ## The key never comes back out
//!
//! [`ai_api_store_key`] takes one inbound. Nothing returns one: the screen renders its
//! state from [`crate::dto::AiApiStatus::key_stored`], which
//! [`codepack_ai_api::keys::has_key`] answers without reading the secret. There is no
//! command that could put a key on the IPC boundary in the outbound direction.

use codepack_ai::handoff;
use tauri::{AppHandle, Emitter};

use crate::dto::{
    AiAnswerResult, AiApiStatus, AiFinishedEvent, AiModelInfo, AiSendPlan, HandoffResult,
    LocalAgentInfo,
};
use crate::error::{CommandError, CommandResult};

/// Event name, matching `ai:finished` in the frontend's `client.ts`.
pub const FINISHED_EVENT: &str = "ai:finished";

/// The agents this build can describe. Advisory: the handoff file works for any tool
/// that reads the folder, including one not listed here.
#[tauri::command]
pub fn list_local_agents() -> Vec<LocalAgentInfo> {
    handoff::AGENTS
        .iter()
        .map(|agent| LocalAgentInfo {
            id: agent.id.to_string(),
            display_name: agent.display_name.to_string(),
            command: agent.command.to_string(),
        })
        .collect()
}

/// Writes the handoff file into the bundle at `result_path`.
///
/// The bundle is extracted first when it is an archive — an agent cannot read a project
/// inside a ZIP — using the same beside-the-archive directory the report-opening
/// commands already use, so a user who has opened the dashboard and then prepares a
/// handoff does not end up with two copies of their bundle.
#[tauri::command]
pub fn prepare_handoff(
    result_path: String,
    agent_id: String,
    question: String,
) -> CommandResult<HandoffResult> {
    let paths = codepack_core::AppPaths::resolve()?;
    prepare_handoff_at(&paths, &result_path, &agent_id, &question)
}

/// [`prepare_handoff`] against an explicit [`codepack_core::AppPaths`].
///
/// This is the command S-2 (audit 2026-09-07) broke outright: the bundle-path check it
/// goes through queried history with `LIMIT 0`, which SQLite defines as "zero rows", so
/// the check rejected *every* path unconditionally — including one this very
/// installation had just produced. Every test on this command before that pass exercised
/// only rejection (an unknown agent, a path never recorded, a path that no longer
/// exists), which a check that rejects everything passes identically to a correct one.
/// `prepare_handoff_answers_for_a_run_this_installation_actually_produced` below is the
/// test that would have caught it: it is the one case none of the others were.
fn prepare_handoff_at(
    paths: &codepack_core::AppPaths,
    result_path: &str,
    agent_id: &str,
    question: &str,
) -> CommandResult<HandoffResult> {
    let agent = handoff::agent(agent_id).ok_or_else(|| {
        let known: Vec<&str> = handoff::AGENTS.iter().map(|entry| entry.id).collect();
        CommandError::new(format!(
            "unknown agent {agent_id:?}. Available: {}",
            known.join(", ")
        ))
    })?;

    let bundle_dir = crate::commands::export::extracted_bundle_dir_at(paths, result_path)?;
    let prepared = handoff::prepare(&bundle_dir, agent, question).map_err(CommandError::new)?;

    Ok(HandoffResult {
        path: prepared.path.display().to_string(),
        working_dir: prepared.working_dir.display().to_string(),
        command: prepared.command,
        agent_name: agent.display_name.to_string(),
    })
}

/// What the settings screen needs in order to decide what to offer.
#[tauri::command]
pub fn ai_api_status() -> CommandResult<AiApiStatus> {
    let paths = codepack_core::AppPaths::resolve()?;
    let config = codepack_core::config::load(&paths);
    status_for(&config)
}

fn status_for(config: &codepack_core::config::Config) -> CommandResult<AiApiStatus> {
    let provider_id = config.normalized_ai_api_provider().to_string();
    let provider = codepack_ai_api::providers::resolve(&provider_id).map_err(CommandError::new)?;

    Ok(AiApiStatus {
        provider: provider_id.clone(),
        provider_display_name: provider.display_name().to_string(),
        enabled: config.ai_api_enabled,
        key_stored: codepack_ai_api::keys::has_key(&provider_id),
        known_models: provider
            .known_models()
            .iter()
            .map(|model| AiModelInfo {
                id: model.id.to_string(),
                display_name: model.display_name.to_string(),
                context_tokens: model.context_tokens,
            })
            .collect(),
    })
}

/// Describes what sending this bundle would mean, without sending it.
///
/// The confirmation step. Nothing here reads a key or opens a connection, so the screen
/// can show the plan for a bundle the user has not decided about yet — including one they
/// will decide against because of what this tells them.
#[tauri::command]
pub fn ai_api_plan(result_path: String, model: Option<String>) -> CommandResult<AiSendPlan> {
    let paths = codepack_core::AppPaths::resolve()?;
    let config = codepack_core::config::load(&paths);
    let bundle_dir = crate::commands::export::extracted_bundle_dir_at(&paths, &result_path)?;

    let provider_id = config.normalized_ai_api_provider().to_string();
    let provider = codepack_ai_api::providers::resolve(&provider_id).map_err(CommandError::new)?;
    let model = resolve_model(model.as_deref(), &config, provider.as_ref())?;

    let plan = codepack_ai_api::plan::build_plan(&bundle_dir, provider.as_ref(), &model)
        .map_err(CommandError::new)?;

    // Asked without the override, so the screen learns whether the guard fires at all.
    let refusal = plan.check(config.ai_api_enabled, false).err();
    let overridable = matches!(
        refusal.as_ref(),
        Some(codepack_ai_api::Refusal::CriticalFindings { .. })
    );

    Ok(AiSendPlan {
        provider: plan.provider.clone(),
        model: plan.model.clone(),
        context_files: plan.context_files,
        context_bytes: plan.context_bytes,
        context_bytes_display: codepack_tokens::format_bytes(plan.context_bytes),
        estimated_tokens: plan.estimated_tokens,
        critical_findings: plan.critical_findings,
        exceeds_context: plan.exceeds_context(),
        refusal: refusal.map(|refusal| refusal.to_string()),
        overridable,
    })
}

/// Sends, on a background thread, and returns a run id.
///
/// Mirrors `commands::sanitize`'s pattern for the same reason: a provider round trip on
/// a large bundle takes from seconds to minutes, and a blocking command would freeze the
/// window for all of it. Unlike an export there is **no cancellation**: `ureq` offers no
/// handle to interrupt a request in flight, so a cancel button here would be a promise
/// the backend cannot keep. Recorded as known debt rather than faked.
#[tauri::command]
pub fn ai_api_ask(
    app: AppHandle,
    result_path: String,
    question: String,
    model: Option<String>,
    override_critical: bool,
) -> CommandResult<String> {
    let paths = codepack_core::AppPaths::resolve()?;
    let config = codepack_core::config::load(&paths);

    // Resolved on this thread so a configuration problem is this call's own error,
    // where the screen can show it beside the button, rather than arriving later as a
    // finished-with-error event for a send that never started.
    let bundle_dir = crate::commands::export::extracted_bundle_dir_at(&paths, &result_path)?;
    let provider_id = config.normalized_ai_api_provider().to_string();
    let provider = codepack_ai_api::providers::resolve(&provider_id).map_err(CommandError::new)?;
    let model = resolve_model(model.as_deref(), &config, provider.as_ref())?;
    let enabled = config.ai_api_enabled;

    let run_id = new_run_id();
    let thread_run_id = run_id.clone();

    std::thread::spawn(move || {
        let outcome = codepack_ai_api::ask(
            &bundle_dir,
            &provider_id,
            &model,
            &question,
            enabled,
            override_critical,
        );

        let event = match outcome {
            Ok(answer) => AiFinishedEvent {
                run_id: thread_run_id,
                answer: Some(AiAnswerResult {
                    text: answer.text,
                    model: answer.model,
                    input_tokens: answer.input_tokens,
                    output_tokens: answer.output_tokens,
                    stopped_early: answer.stopped_early,
                    answer_file: bundle_dir
                        .join(codepack_ai_api::plan::ANSWER_FILE)
                        .display()
                        .to_string(),
                }),
                error: None,
            },
            // Through `CommandError`, which redacts: an `AiError` can name a bundle path,
            // and a provider's own message is prose this window is about to display.
            Err(error) => AiFinishedEvent {
                run_id: thread_run_id,
                answer: None,
                error: Some(CommandError::new(error).message),
            },
        };
        let _ = app.emit(FINISHED_EVENT, event);
    });

    Ok(run_id)
}

/// Puts a key in the OS credential store.
///
/// The only command in this crate that takes a secret, and it keeps it for exactly as
/// long as the call: the value goes to [`codepack_ai_api::keys::store_key`] and is not
/// logged, echoed back, or placed in any `Serialize` type on the way.
#[tauri::command]
pub fn ai_api_store_key(key: String) -> CommandResult<AiApiStatus> {
    // Before anything is resolved or read: an empty key is a bad argument, and refusing
    // it must not depend on a readable settings directory or a reachable key store.
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(CommandError::new("no key was given"));
    }

    let paths = codepack_core::AppPaths::resolve()?;
    let config = codepack_core::config::load(&paths);
    let provider_id = config.normalized_ai_api_provider().to_string();

    codepack_ai_api::keys::store_key(&provider_id, trimmed).map_err(CommandError::new)?;

    // The fresh status, so the screen re-renders from what the store says rather than
    // from an assumption that the write worked.
    status_for(&config)
}

#[tauri::command]
pub fn ai_api_clear_key() -> CommandResult<AiApiStatus> {
    let paths = codepack_core::AppPaths::resolve()?;
    let config = codepack_core::config::load(&paths);
    let provider_id = config.normalized_ai_api_provider().to_string();

    codepack_ai_api::keys::clear_key(&provider_id).map_err(CommandError::new)?;
    status_for(&config)
}

/// An id for one send, unique enough to tell two of them apart.
///
/// Not `AppState::runs`, which the export and sterile-copy runs use: that registry exists
/// to carry a cancellation token, and this path has nothing to cancel. Registering a run
/// nobody can stop would put a token in a table for the sole purpose of never being read.
fn new_run_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or_default();
    format!("ai-{nanos:x}")
}

/// The model to ask: what the screen chose, then the setting, then the provider's most
/// capable known model.
///
/// The last step is why `Config::ai_api_model` may be empty — `codepack-core` names no
/// vendor's models, so "the best available" resolves only where the provider is known.
fn resolve_model(
    chosen: Option<&str>,
    config: &codepack_core::config::Config,
    provider: &dyn codepack_ai_api::AiProvider,
) -> CommandResult<String> {
    if let Some(model) = chosen.map(str::trim).filter(|id| !id.is_empty()) {
        return Ok(model.to_string());
    }
    if !config.ai_api_model.trim().is_empty() {
        return Ok(config.ai_api_model.clone());
    }
    provider
        .known_models()
        .first()
        .map(|model| model.id.to_string())
        .ok_or_else(|| {
            CommandError::new(format!(
                "provider {} advertises no models, so one has to be chosen",
                provider.id()
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_agent_offered_to_the_ui_can_be_resolved_again() {
        // The frontend sends back an id from this list; one that does not resolve would
        // be a button that always fails.
        for agent in list_local_agents() {
            assert!(handoff::agent(&agent.id).is_some());
            assert!(!agent.display_name.is_empty());
            assert!(!agent.command.is_empty());
        }
    }

    #[test]
    fn an_unknown_agent_is_refused_with_the_alternatives_named() {
        let dir = tempfile::tempdir().unwrap();
        let error = prepare_handoff(
            dir.path().display().to_string(),
            "not-an-agent".to_string(),
            "q".to_string(),
        )
        .unwrap_err();

        assert!(error.message.contains("not-an-agent"), "{}", error.message);
        assert!(error.message.contains("claude-code"), "{}", error.message);
    }

    #[test]
    fn preparing_writes_the_file_into_an_already_extracted_bundle() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("AI_CONTEXT")).unwrap();

        // Through `handoff::prepare` rather than the command: since audit No. 6 the
        // command first checks that the path is an export this installation produced,
        // and a temporary directory is deliberately not one. What is under test here is
        // what gets written into a bundle, which is this half.
        let agent = handoff::agent("claude-code").expect("a known agent");
        let prepared = handoff::prepare(dir.path(), agent, "review the auth flow").unwrap();

        assert_eq!(prepared.working_dir, dir.path());
        assert_eq!(prepared.command, "claude");
        let body = std::fs::read_to_string(&prepared.path).unwrap();
        assert!(body.contains("review the auth flow"));
    }

    #[test]
    fn a_result_path_that_no_longer_exists_is_reported_rather_than_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone.zip");

        let error = prepare_handoff(
            missing.display().to_string(),
            "claude-code".to_string(),
            String::new(),
        )
        .unwrap_err();
        assert!(
            error.message.contains("no longer where it was recorded"),
            "{}",
            error.message
        );
    }

    /// The guard the command gained: a directory nobody exported is not a bundle.
    #[test]
    fn preparing_refuses_a_path_no_run_produced() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("AI_CONTEXT")).unwrap();

        let error = prepare_handoff(
            dir.path().display().to_string(),
            "claude-code".to_string(),
            "review the auth flow".to_string(),
        )
        .expect_err("an unrecorded path must not be opened");
        assert!(
            format!("{error:?}").contains("not an export this installation produced"),
            "{error:?}"
        );
    }

    /// The end-to-end case every test above was missing (audit 2026-09-07, S-1/S-2/T-1):
    /// a run this installation genuinely produced must be *accepted*, not just correctly
    /// rejected when it is not one. Isolated `AppPaths` and a temporary history database
    /// (audit S-7/T-2), so this never touches whoever runs `cargo test`'s real profile.
    #[test]
    fn the_status_never_carries_the_key_itself() {
        // The serialized shape is what crosses the IPC boundary into a webview. There is
        // no field a key could occupy, and this is what stops one being added for
        // convenience — `key_stored` is a bool for exactly this reason.
        let status = AiApiStatus {
            provider: "anthropic".to_string(),
            provider_display_name: "Anthropic".to_string(),
            enabled: true,
            key_stored: true,
            known_models: Vec::new(),
        };
        let json = serde_json::to_value(&status).unwrap();
        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "enabled",
                "key_stored",
                "known_models",
                "provider",
                "provider_display_name",
            ]
        );
    }

    #[test]
    fn the_status_reports_the_integration_off_for_a_default_configuration() {
        // What a fresh installation looks like, and the reason a fresh installation
        // cannot reach the network: the screen renders a switch, not a send button.
        let status = status_for(&codepack_core::config::Config::default()).unwrap();
        assert!(!status.enabled);
        assert_eq!(status.provider, "anthropic");
        assert!(
            !status.known_models.is_empty(),
            "the screen needs something to offer"
        );
    }

    #[test]
    fn an_empty_key_is_refused_rather_than_stored() {
        // Storing whitespace would make `key_stored` report a key that cannot work, and
        // the screen would show a ready state that fails on the first send.
        let error = ai_api_store_key("   ".to_string()).unwrap_err();
        assert!(error.message.contains("no key"), "{}", error.message);
    }

    #[test]
    fn planning_refuses_a_bundle_no_run_produced() {
        // The same boundary every other bundle command crosses: a path the webview
        // invents is not an export, however well formed it looks.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("AI_CONTEXT")).unwrap();

        let error = ai_api_plan(dir.path().display().to_string(), None)
            .expect_err("an unrecorded path must not be read");
        assert!(
            format!("{error:?}").contains("not an export this installation produced"),
            "{error:?}"
        );
    }

    #[test]
    fn the_bundle_boundary_ask_uses_refuses_a_path_no_run_produced() {
        // `ai_api_ask` calls this on the *calling* thread, before spawning anything, so
        // a refusal is that call's own error rather than an event arriving later about a
        // send which never began. Exercised through the boundary itself because the
        // command needs an `AppHandle` a unit test has no way to mint.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("AI_CONTEXT")).unwrap();

        let paths = codepack_core::AppPaths::resolve().unwrap();
        let error = crate::commands::export::extracted_bundle_dir_at(
            &paths,
            &dir.path().display().to_string(),
        )
        .expect_err("an unrecorded path must not be read");
        assert!(
            format!("{error:?}").contains("not an export this installation produced"),
            "{error:?}"
        );
    }

    #[test]
    fn the_model_falls_back_to_the_most_capable_one_this_build_knows() {
        let provider = codepack_ai_api::providers::resolve("anthropic").unwrap();
        let config = codepack_core::config::Config::default();

        // Nothing chosen, nothing stored: the provider's first model, which its own
        // contract says is the most capable.
        let resolved = resolve_model(None, &config, provider.as_ref()).unwrap();
        assert_eq!(resolved, provider.known_models()[0].id);

        // A model the screen chose wins over both.
        assert_eq!(
            resolve_model(Some("a-model-from-next-year"), &config, provider.as_ref()).unwrap(),
            "a-model-from-next-year"
        );

        // And a blank choice is not a choice.
        assert_eq!(
            resolve_model(Some("  "), &config, provider.as_ref()).unwrap(),
            provider.known_models()[0].id
        );
    }

    #[test]
    fn prepare_handoff_answers_for_a_run_this_installation_actually_produced() {
        let history_root = tempfile::tempdir().unwrap();
        let paths = codepack_core::AppPaths::for_root(history_root.path());

        let bundle = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(bundle.path().join("AI_CONTEXT")).unwrap();

        let mut connection = crate::commands::open_database_at(&paths).unwrap();
        let project =
            codepack_storage::find_or_create_project(&connection, "/tmp/project", "project", None)
                .unwrap();
        codepack_storage::record_export_run(
            &mut connection,
            codepack_storage::NewExportRun {
                project_id: project,
                started_at: 10,
                finished_at: Some(11),
                profile: Some("full".to_string()),
                safe_mode: Some("safe".to_string()),
                diff_mode: Some("all".to_string()),
                files_copied: Some(1),
                bytes_total: Some(10),
                tokens_est: Some(1),
                redacted_count: None,
                cancelled: false,
                result_path: Some(bundle.path().display().to_string()),
            },
            &[],
            &[],
            &[],
            None,
        )
        .unwrap();

        let result = prepare_handoff_at(
            &paths,
            &bundle.path().display().to_string(),
            "claude-code",
            "review the auth flow",
        )
        .expect("a run this installation produced must be accepted");

        assert_eq!(result.command, "claude");
        assert!(
            std::fs::read_to_string(&result.path)
                .unwrap()
                .contains("review the auth flow")
        );
    }
}
