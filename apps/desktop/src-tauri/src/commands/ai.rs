//! Handing a finished bundle to a coding agent running on this machine.
//!
//! This is stage S13's offline path, and the only one the interface offers. The API path
//! exists in `codepack-ai` behind its `api` feature; this crate takes that dependency
//! with `default-features = false`, so the desktop binary contains **no HTTP client and
//! no credential store at all** — invariant I1 held by what is linked, not by what the
//! code chooses to call.
//!
//! Nothing is launched. The command writes `AI_HANDOFF.md` into the bundle and returns
//! the command to run there; the user starts their own agent, in their own terminal,
//! where they can see what it does. Spawning a process would also need a capability the
//! webview deliberately does not have.

use codepack_ai::handoff;

use crate::dto::{HandoffResult, LocalAgentInfo};
use crate::error::{CommandError, CommandResult};

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
