//! Where output goes, and what shape it has.
//!
//! Two rules hold everywhere in this binary:
//!
//! * **`--json` output is the only thing on stdout.** Progress, warnings and errors go
//!   to stderr. Otherwise `codepack export … --json | jq` breaks the moment the export
//!   logs anything, which is always. In human mode stdout carries the report and stderr
//!   still carries progress, so redirecting either stream stays useful.
//! * **Machine output is versioned.** `docs/__arch__/ROADMAP.md` §3 requires `--json` to be "stable
//!   by schema, versioned"; every payload therefore carries [`JSON_SCHEMA_VERSION`]
//!   next to a `command` discriminator, so a consumer can tell both *what* it is
//!   reading and *which* revision of it.

use std::io::Write;

use serde::Serialize;

/// Bumped when a `--json` payload changes in a way a consumer could notice: a removed
/// or renamed field, a changed type, a changed meaning. Adding a new optional field
/// does not require a bump, matching how `CONFIG_SCHEMA_VERSION` is governed.
pub(crate) const JSON_SCHEMA_VERSION: u32 = 1;

/// Wraps a command's payload with the two fields every consumer needs before it can
/// interpret the rest.
#[derive(Debug, Serialize)]
pub(crate) struct Envelope<T: Serialize> {
    pub schema_version: u32,
    pub command: &'static str,
    #[serde(flatten)]
    pub payload: T,
}

impl<T: Serialize> Envelope<T> {
    pub(crate) fn new(command: &'static str, payload: T) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            command,
            payload,
        }
    }
}

/// How a command should present itself. Threaded explicitly rather than read from a
/// global so tests can exercise both modes without touching process state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Human,
    Json,
}

impl Format {
    pub(crate) fn from_flag(json: bool) -> Self {
        if json { Self::Json } else { Self::Human }
    }

    pub(crate) fn is_json(self) -> bool {
        self == Self::Json
    }
}

/// Renders an envelope as pretty JSON with a trailing newline.
///
/// Pretty rather than compact: this output is read by people at least as often as by
/// programs, `jq` does not care, and a diff between two runs is legible.
pub(crate) fn render_json<T: Serialize>(
    command: &'static str,
    payload: T,
) -> serde_json::Result<String> {
    let mut text = serde_json::to_string_pretty(&Envelope::new(command, payload))?;
    text.push('\n');
    Ok(text)
}

/// Prints machine output to stdout.
pub(crate) fn emit_json<T: Serialize>(
    command: &'static str,
    payload: T,
) -> crate::error::Result<()> {
    let text = render_json(command, payload).map_err(|source| {
        crate::error::CliError::message(format!("cannot serialize output: {source}"))
    })?;
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(text.as_bytes()).map_err(|source| {
        crate::error::CliError::message(format!("cannot write to stdout: {source}"))
    })
}

/// Prints a human-facing line to stdout.
pub(crate) fn line(text: impl AsRef<str>) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{}", text.as_ref());
}

/// Prints a diagnostic to stderr. Safe to call in `--json` mode: it never contaminates
/// the machine-readable stream.
pub(crate) fn note(text: impl AsRef<str>) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{}", text.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Payload {
        files: usize,
    }

    #[test]
    fn every_payload_carries_its_schema_version_and_command() {
        let text = render_json("preview", Payload { files: 3 }).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(parsed["schema_version"], JSON_SCHEMA_VERSION);
        assert_eq!(parsed["command"], "preview");
        assert_eq!(parsed["files"], 3);
    }

    #[test]
    fn the_payload_is_flattened_rather_than_nested() {
        // A consumer reading `.files` must not have to know whether it lives under a
        // `payload` key; flattening is part of the contract, not a formatting choice.
        let text = render_json("preview", Payload { files: 1 }).unwrap();
        assert!(!text.contains("\"payload\""));
    }

    #[test]
    fn output_ends_with_a_newline_so_it_composes_with_line_oriented_tools() {
        assert!(
            render_json("preview", Payload { files: 1 })
                .unwrap()
                .ends_with('\n')
        );
    }

    #[test]
    fn format_maps_the_flag_both_ways() {
        assert_eq!(Format::from_flag(true), Format::Json);
        assert_eq!(Format::from_flag(false), Format::Human);
        assert!(Format::from_flag(true).is_json());
    }
}
