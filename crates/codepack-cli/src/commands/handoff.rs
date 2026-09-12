//! `codepack handoff <bundle>` — pointing a local coding agent at an export.
//!
//! ## Why this is not a network feature
//!
//! Claude Code and Codex run on this machine and read this filesystem. Sending them a
//! bundle over HTTP would be absurd, and it would drag the whole apparatus of stage
//! S13's API path — a key, a credential store, a transport — into a job that needs a
//! text file. So this command writes `AI_HANDOFF.md` into the bundle and prints the
//! command to run there. Invariant I1 is untouched: nothing here can reach the network,
//! because this binary links no client at all (`codepack-ai` is taken with
//! `default-features = false`).
//!
//! ## Why it does not launch the agent
//!
//! Spawning a process on the user's behalf would be a new capability for a gain of one
//! keystroke, and the user starting their own agent in their own terminal can see what
//! it does. The command is printed, ready to paste.
//!
//! ## Why a ZIP is extracted beside itself
//!
//! An agent cannot read a project inside an archive, and a temporary directory would be
//! gone before it started. So a `.zip` is unpacked next to itself into
//! `<name>_extracted` — the same place and the same name the desktop app uses, so both
//! front ends leave the machine in the same state. That logic moved to
//! [`crate::commands::bundle`] when `ask` came to need exactly the same thing.

use codepack_ai::handoff::{self, LocalAgent};
use serde::Serialize;

use crate::cli::HandoffArgs;
use crate::commands::bundle;
use crate::error::{CliError, Result};
use crate::exit::Outcome;
use crate::output::{self, Format};

#[derive(Debug, Serialize)]
pub(crate) struct HandoffReport {
    /// The bundle as the user named it.
    pub bundle: String,
    /// The directory the agent should be started in — the same thing as `bundle` unless
    /// an archive had to be unpacked first.
    pub working_dir: String,
    /// Whether an archive was unpacked to produce `working_dir`.
    pub extracted: bool,
    pub agent: String,
    pub agent_name: String,
    /// The file that was written.
    pub handoff_file: String,
    /// The command to run in `working_dir`.
    pub command: String,
}

pub(crate) fn run(args: &HandoffArgs, format: Format) -> Result<Outcome> {
    // The stored agent and question come from global settings: this command takes no
    // project directory, so there is no `.codepack.toml` layer to resolve. Explicit
    // flags win, as everywhere else in this binary.
    let app_paths = codepack_core::AppPaths::resolve()?;
    let config = codepack_core::config::load(&app_paths);

    let agent_id = args
        .agent
        .clone()
        .unwrap_or_else(|| config.normalized_ai_handoff_agent().to_string());
    let agent = resolve_agent(&agent_id)?;

    let question = args
        .question
        .clone()
        .unwrap_or_else(|| config.ai_handoff_question.clone());

    let opened = bundle::open_bundle(&args.bundle)?;
    let prepared = handoff::prepare(&opened.directory, agent, &question)
        .map_err(|error| CliError::message(error.to_string()))?;

    let report = HandoffReport {
        bundle: args.bundle.display().to_string(),
        working_dir: prepared.working_dir.display().to_string(),
        extracted: opened.extracted,
        agent: agent.id.to_string(),
        agent_name: agent.display_name.to_string(),
        handoff_file: prepared.path.display().to_string(),
        command: prepared.command,
    };

    if format.is_json() {
        output::emit_json("handoff", &report)?;
    } else {
        print_human(&report);
    }
    Ok(Outcome::Success)
}

/// An unknown agent is an error naming the alternatives, never a silent fallback: a
/// handoff addressed to an agent the user did not choose is a file they will not read.
fn resolve_agent(id: &str) -> Result<LocalAgent> {
    handoff::agent(id).ok_or_else(|| {
        let known: Vec<&str> = handoff::AGENTS.iter().map(|entry| entry.id).collect();
        CliError::message(format!(
            "unknown agent {id:?}. Available: {}",
            known.join(", ")
        ))
    })
}

fn print_human(report: &HandoffReport) {
    output::line(format!("Prepared for: {}", report.agent_name));
    output::line(format!("Wrote:        {}", report.handoff_file));
    if report.extracted {
        output::line(format!("Extracted to: {}", report.working_dir));
    }
    output::line("");
    output::line("Run the agent there:");
    output::line("");
    output::line(format!("  cd {}", report.working_dir));
    output::line(format!("  {}", report.command));
    output::line("");
    output::line("Nothing was sent anywhere; the agent reads the folder itself.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_agent_resolves_and_an_unknown_one_lists_the_alternatives() {
        assert_eq!(resolve_agent("claude-code").unwrap().id, "claude-code");

        let error = resolve_agent("gpt-typo").unwrap_err().to_string();
        assert!(error.contains("gpt-typo"), "{error}");
        assert!(error.contains("claude-code"), "{error}");
    }
}
