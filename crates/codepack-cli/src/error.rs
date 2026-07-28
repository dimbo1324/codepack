//! The CLI's error type.
//!
//! `.ai/project/12-domain-rules.md` reserves `anyhow` for binaries. This one uses
//! `thiserror` anyway, because two call sites match on the variant rather than on the
//! message (`commands::mod`'s `NotADirectory`), and because
//! [`CliError::ProjectConfigSyntax`] has to control its own rendering to keep a
//! configuration file's contents out of the message. `anyhow` gives neither without
//! downcasting. The deviation is small and local; the rule's intent — binaries should
//! not force typed errors on their callers — is not at stake in a crate with no
//! callers.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub(crate) enum CliError {
    #[error("{0}")]
    Message(String),

    #[error("{path} is not a directory")]
    NotADirectory { path: PathBuf },

    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Rendered from the error's own message and span, never from its `Display`.
    ///
    /// `toml::de::Error`'s `Display` embeds the offending source line verbatim, value
    /// included, and `deny_unknown_fields` guarantees that any non-schema key is echoed
    /// that way. That would make this the one path in the binary that prints raw
    /// repository file content — a `.codepack.toml` carrying a stray
    /// `api_token = "…"` would have the value read back on stderr and into a CI log.
    /// Passing it through `redact_secrets` is not enough: the redactor keys on its own
    /// keyword list, and an arbitrary key name in an arbitrary file need not be on it.
    /// Reporting the position instead of the text says everything the user needs to
    /// find the line themselves.
    #[error("{path} is not valid TOML{}: {}", .span, .message)]
    ProjectConfigSyntax {
        path: PathBuf,
        span: String,
        message: String,
    },

    #[error(transparent)]
    Core(#[from] codepack_core::CoreError),

    #[error(transparent)]
    Engine(#[from] codepack_engine::EngineError),

    #[error(transparent)]
    Security(#[from] codepack_security::SecurityError),

    #[error(transparent)]
    Storage(#[from] codepack_storage::StorageError),

    #[error(transparent)]
    Sanitize(#[from] codepack_sanitize::SanitizeError),
}

impl CliError {
    pub(crate) fn message(text: impl Into<String>) -> Self {
        Self::Message(text.into())
    }
}

pub(crate) type Result<T> = std::result::Result<T, CliError>;
