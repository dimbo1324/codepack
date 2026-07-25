//! The one error type every `#[tauri::command]` rejects with.
//!
//! ## Why commands never return a bare string
//!
//! Tauri will happily serialize a `String` error, and the frontend would render it. But
//! a bare string has no shape, so every call site invents its own way to display it, and
//! nothing stops an error message from being built by `format!`-ing a value that came
//! out of a scanned file. This crate's whole purpose is keeping secrets out of places
//! they should not be, and an error surface is such a place — invariant I3 does not stop
//! at report files.
//!
//! [`CommandError`] therefore has exactly one field, and its constructor routes the
//! message through the same redaction the reports use.

use serde::Serialize;

/// A structured, already-redacted failure, matching `CommandError` in
/// `apps/desktop/ui/src/lib/api/types.ts`.
#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub message: String,
}

impl CommandError {
    /// Builds an error from anything displayable, redacting it on the way through.
    ///
    /// The redaction is not decoration. An I/O error names the path it failed on, and a
    /// path can be `…/config/.env`; a parse error can quote the line it choked on. Both
    /// reach the user's screen, and a screenshot of a crash is one of the most commonly
    /// shared artifacts there is.
    pub fn new(message: impl std::fmt::Display) -> Self {
        Self {
            message: codepack_security::patterns::keyword::redacted_line(&message.to_string()),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CommandError {}

/// Every fallible command returns this.
pub type CommandResult<T> = std::result::Result<T, CommandError>;

macro_rules! from_error {
    ($($source:ty),+ $(,)?) => {
        $(
            impl From<$source> for CommandError {
                fn from(error: $source) -> Self {
                    Self::new(error)
                }
            }
        )+
    };
}

from_error!(
    codepack_core::CoreError,
    codepack_engine::EngineError,
    codepack_scanner::ScannerError,
    codepack_security::SecurityError,
    codepack_storage::StorageError,
    std::io::Error,
    serde_json::Error,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_in_an_error_message_is_redacted_before_it_can_reach_the_screen() {
        // The realistic shape: an I/O failure that names the file, where the "file" is a
        // value someone pasted into a config. A crash screenshot is a shared artifact.
        let error = CommandError::new("failed to read API_KEY=sk-super-secret-value-here");
        assert!(
            !error.message.contains("sk-super-secret-value-here"),
            "secret survived into the error surface: {}",
            error.message
        );
        assert!(error.message.contains("<REDACTED>"));
    }

    #[test]
    fn an_ordinary_error_message_survives_intact() {
        // Over-redaction would make errors useless, so the common case must pass through.
        let error = CommandError::new("project directory does not exist");
        assert_eq!(error.message, "project directory does not exist");
    }

    #[test]
    fn the_serialized_shape_is_the_one_the_frontend_declares() {
        let json = serde_json::to_value(CommandError::new("boom")).unwrap();
        assert_eq!(json["message"], "boom");
        assert_eq!(json.as_object().unwrap().len(), 1);
    }

    #[test]
    fn io_errors_convert_and_are_redacted_on_the_way() {
        let io = std::io::Error::other("open failed for PASSWORD=hunter2placeholder");
        let error: CommandError = io.into();
        assert!(!error.message.contains("hunter2placeholder"));
    }
}
