//! Handing a bundle to a coding agent that already runs on this machine.
//!
//! Claude Code and Codex are not API models — they are agents with their own filesystem
//! access. Shipping them a bundle over HTTP would be absurd: they can open the folder.
//! So this path makes no network call, needs no API key, and leaves invariant I1
//! completely untouched. It writes one file the agent will find, and hands back the
//! command to run.
//!
//! That makes it the cheaper and safer half of stage S13, and the one most users will
//! actually reach for. It is deliberately implemented first-class rather than as a
//! documentation note.

use std::path::{Path, PathBuf};

use crate::error::AiError;

/// Written beside the extracted bundle. Named so an agent scanning the directory sees
/// what it is without being told.
pub const HANDOFF_FILE: &str = "AI_HANDOFF.md";

/// A local agent this build knows how to describe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalAgent {
    pub id: &'static str,
    pub display_name: &'static str,
    /// The command to run from inside the bundle directory.
    pub command: &'static str,
}

/// Agents offered in the interface.
///
/// This list is advisory — the handoff file works with any agent, including one not
/// listed, because all it does is put a prompt where a filesystem-reading tool will see
/// it. Nothing here shells out, so an entry cannot be wrong in a way that breaks a run.
pub const AGENTS: &[LocalAgent] = &[
    LocalAgent {
        id: "claude-code",
        display_name: "Claude Code",
        command: "claude",
    },
    LocalAgent {
        id: "codex",
        display_name: "Codex CLI",
        command: "codex",
    },
];

pub fn agent(id: &str) -> Option<LocalAgent> {
    AGENTS.iter().copied().find(|agent| agent.id == id)
}

/// What the caller needs to show the user after a handoff is prepared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    /// The file that was written.
    pub path: PathBuf,
    /// The directory to run the agent in.
    pub working_dir: PathBuf,
    /// The command to run there, ready to copy.
    pub command: String,
}

/// Write the handoff file into an extracted bundle.
///
/// Deliberately does **not** launch anything. Spawning a process would need a capability
/// the webview does not have and this project has been careful not to grant — and the
/// user launching their own agent, in a terminal they can see, is both safer and how
/// they already work.
pub fn prepare(bundle_dir: &Path, agent: LocalAgent, question: &str) -> Result<Handoff, AiError> {
    let path = bundle_dir.join(HANDOFF_FILE);
    let body = handoff_body(agent, question);

    std::fs::write(&path, body).map_err(|source| AiError::Bundle {
        path: path.clone(),
        source,
    })?;

    Ok(Handoff {
        path,
        working_dir: bundle_dir.to_path_buf(),
        command: agent.command.to_string(),
    })
}

fn handoff_body(agent: LocalAgent, question: &str) -> String {
    let question = if question.trim().is_empty() {
        "Review this project and report what you find."
    } else {
        question.trim()
    };

    format!(
        "# Handoff to {name}\n\
         \n\
         This directory is an exported snapshot of a software project, produced by \
         codepack. It is partial by design: files may have been excluded for size or on \
         safety grounds, so treat a missing file as \"not exported\", not as \"not in the \
         project\".\n\
         \n\
         ## Where to start\n\
         \n\
         - `AI_CONTEXT/` — one file per aspect of the project, smallest useful entry point\n\
         - `AI_CONTEXT/00_PROJECT_OVERVIEW.md` — what this project is\n\
         - `AI_CONTEXT/04_KEY_FILES.md` — the files worth reading first\n\
         - `06_security_scan.txt` — what the scanner found, if it ran\n\
         - `REPORT_DASHBOARD.html` — every report, navigable\n\
         \n\
         ## Task\n\
         \n\
         {question}\n",
        name = agent.display_name,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_agent_resolves_and_an_unknown_one_does_not() {
        assert_eq!(agent("claude-code").unwrap().command, "claude");
        assert!(agent("not-an-agent").is_none());
    }

    #[test]
    fn preparing_writes_the_file_and_reports_where_to_run() {
        let dir = tempfile::tempdir().unwrap();
        let handoff = prepare(dir.path(), agent("claude-code").unwrap(), "find bugs").unwrap();

        assert_eq!(handoff.working_dir, dir.path());
        assert_eq!(handoff.command, "claude");
        let body = std::fs::read_to_string(&handoff.path).unwrap();
        assert!(body.contains("find bugs"));
        assert!(body.contains("Claude Code"));
    }

    #[test]
    fn an_empty_question_still_produces_a_usable_task() {
        // A blank prompt handed to an agent is a wasted run; a sensible default is not.
        let dir = tempfile::tempdir().unwrap();
        let handoff = prepare(dir.path(), agent("codex").unwrap(), "   ").unwrap();
        let body = std::fs::read_to_string(&handoff.path).unwrap();
        assert!(body.contains("Review this project"));
    }

    #[test]
    fn the_snapshot_is_described_as_partial() {
        // An agent that assumes a missing file means the project lacks it will draw
        // confident wrong conclusions, so the file says so up front.
        let dir = tempfile::tempdir().unwrap();
        let handoff = prepare(dir.path(), AGENTS[0], "q").unwrap();
        let body = std::fs::read_to_string(&handoff.path).unwrap();
        assert!(body.contains("partial by design"));
    }

    #[test]
    fn every_listed_agent_resolves_by_its_own_id() {
        for entry in AGENTS {
            assert_eq!(agent(entry.id), Some(*entry));
        }
    }

    #[test]
    fn the_agents_here_are_exactly_the_ones_config_will_accept() {
        // The ids are duplicated on purpose: `Config` must normalize the stored value
        // and cannot depend on this crate (the dependency points `ai → core`). A split
        // like that is only safe while something fails when it drifts — a setting
        // naming an agent this list does not describe would resolve to nothing at all.
        let here: Vec<&str> = AGENTS.iter().map(|entry| entry.id).collect();
        assert_eq!(
            here,
            codepack_core::config::LOCAL_AI_AGENTS.to_vec(),
            "codepack-ai::handoff::AGENTS and Config's LOCAL_AI_AGENTS have drifted"
        );
        assert!(agent(codepack_core::config::DEFAULT_LOCAL_AI_AGENT).is_some());
    }
}
