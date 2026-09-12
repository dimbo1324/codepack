//! `codepack key set|status|clear` — the API key, and the one place it is allowed to
//! live.
//!
//! ## Why the key is read from stdin and never from a flag
//!
//! A flag would be the obvious shape — `codepack key set --value sk-…` — and it is the
//! wrong one. An argument is visible to every other process on the machine for as long
//! as this one runs (`ps`, Task Manager's command-line column), and the shell writes it
//! into `~/.bash_history` or `ConsoleHost_history.txt`, where it stays indefinitely. A
//! credential that reaches either place has leaked, and neither is something this
//! program can clean up afterwards.
//!
//! So the key arrives on stdin. That is what a script or a password manager needs
//! (`pass show anthropic | codepack key set`), and it keeps the value out of both places
//! above. Nothing in this module logs it, returns it, or puts it in a struct that derives
//! `Serialize`; [`codepack_ai_api::keys`] is the only code that touches it afterwards,
//! and it puts the value in the OS credential store and nowhere else.
//!
//! ## Why there is no `codepack key show`
//!
//! Nothing in this product needs to display a stored key, and a command that printed one
//! would exist mainly to be run in a shared terminal. `status` answers the question
//! people actually have — is a key there — through
//! [`codepack_ai_api::keys::has_key`], which never reads the secret.

use std::io::{BufRead, IsTerminal, Write};

use serde::Serialize;

use crate::cli::{KeyArgs, KeyCommand};
use crate::error::{CliError, Result};
use crate::exit::Outcome;
use crate::output::{self, Format};

#[derive(Debug, Serialize)]
pub(crate) struct KeyReport {
    /// `set`, `status` or `clear`.
    pub action: &'static str,
    pub provider: String,
    /// Whether a key is stored **after** this command ran. The only thing this binary
    /// ever says about a key's content.
    pub stored: bool,
}

pub(crate) fn run(args: &KeyArgs, format: Format) -> Result<Outcome> {
    let provider = resolve_provider(args.provider.as_deref())?;

    let report = match &args.command {
        KeyCommand::Set => set(&provider)?,
        KeyCommand::Status => KeyReport {
            action: "status",
            stored: codepack_ai_api::keys::has_key(&provider),
            provider,
        },
        KeyCommand::Clear => {
            codepack_ai_api::keys::clear_key(&provider)
                .map_err(|error| CliError::message(error.to_string()))?;
            KeyReport {
                action: "clear",
                // Read back rather than assumed. `clear_key` treats "there was none" as
                // success, so the honest report is whatever the store says now.
                stored: codepack_ai_api::keys::has_key(&provider),
                provider,
            }
        }
    };

    if format.is_json() {
        output::emit_json("key", &report)?;
    } else {
        print_human(&report);
    }
    Ok(Outcome::Success)
}

/// The provider a key belongs to: the flag, else the configured one.
///
/// Validated against the providers this build implements, because a key stored under a
/// name nothing resolves is a key the user will believe is in place.
fn resolve_provider(flag: Option<&str>) -> Result<String> {
    let id = match flag {
        Some(id) => id.to_string(),
        None => {
            let app_paths = codepack_core::AppPaths::resolve()?;
            codepack_core::config::load(&app_paths)
                .normalized_ai_api_provider()
                .to_string()
        }
    };

    if codepack_ai_api::providers::resolve(&id).is_err() {
        let known: Vec<&str> = codepack_ai_api::providers::all()
            .iter()
            .map(|provider| provider.id())
            .collect();
        return Err(CliError::message(format!(
            "unknown provider {id:?}. Available: {}",
            known.join(", ")
        )));
    }
    Ok(id)
}

fn set(provider: &str) -> Result<KeyReport> {
    let key = read_key_from_stdin()?;
    if key.is_empty() {
        return Err(CliError::message(
            "no key was given on stdin. Pipe one in, or type it when prompted",
        ));
    }

    codepack_ai_api::keys::store_key(provider, &key)
        .map_err(|error| CliError::message(error.to_string()))?;

    Ok(KeyReport {
        action: "set",
        provider: provider.to_string(),
        // Read back through the store, so "stored" means the store agrees rather than
        // meaning `store_key` returned `Ok`.
        stored: codepack_ai_api::keys::has_key(provider),
    })
}

/// One line from stdin.
///
/// **Echo is not switched off, and the prompt says so.** Hiding input properly means
/// either `unsafe` FFI into the console API — which `unsafe_code = "forbid"` rules out
/// workspace-wide and which is not worth an owner exception for one prompt — or a
/// terminal crate, the sort of dependency `.ai/universal/05-security-and-secrets.md`
/// says not to add for one small function. Saying so plainly beats both: somebody who
/// knows the key will appear on screen can pipe it instead, while somebody told it was
/// hidden cannot make that choice.
///
/// The masked field for a person who would rather type than pipe is on the desktop
/// settings screen, which is the front end built for typing.
fn read_key_from_stdin() -> Result<String> {
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        output::note(
            "This terminal will echo the key as you type, and your shell may keep it in \
             scrollback. Ctrl+C and pipe it in instead (pass show anthropic | codepack key \
             set), or use the desktop app's settings screen.",
        );
        output::note("Paste the API key and press Enter:");
        let _ = std::io::stderr().flush();
    }

    let mut line = String::new();
    stdin
        .lock()
        .read_line(&mut line)
        .map_err(|source| CliError::message(format!("cannot read the key from stdin: {source}")))?;

    Ok(line.trim().to_string())
}

fn print_human(report: &KeyReport) {
    match report.action {
        "set" => output::line(format!(
            "Stored a key for {} in this machine's credential store.",
            report.provider
        )),
        "clear" => output::line(format!("No key is stored for {} now.", report.provider)),
        _ if report.stored => output::line(format!("A key is stored for {}.", report.provider)),
        _ => output::line(format!(
            "No key is stored for {}. Set one with: codepack key set",
            report.provider
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_provider_is_refused_with_the_alternatives() {
        let error = resolve_provider(Some("not-a-provider")).unwrap_err();
        assert!(error.to_string().contains("not-a-provider"), "{error}");
        assert!(error.to_string().contains("anthropic"), "{error}");
    }

    #[test]
    fn the_configured_provider_resolves_to_one_this_build_implements() {
        // Whatever the machine's settings happen to say, `normalized_ai_api_provider`
        // guarantees a provider that exists. This asserts the command leans on that
        // rather than on the raw stored string.
        let provider = resolve_provider(None).unwrap();
        assert!(codepack_ai_api::providers::resolve(&provider).is_ok());
    }

    #[test]
    fn the_report_never_carries_the_key_itself() {
        // The serialized shape is what reaches a `--json` consumer and a CI log. There is
        // no field for a key here, and this test is what stops somebody adding one for
        // convenience.
        let json = serde_json::to_string(&KeyReport {
            action: "status",
            provider: "anthropic".to_string(),
            stored: true,
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"action":"status","provider":"anthropic","stored":true}"#
        );
    }
}
