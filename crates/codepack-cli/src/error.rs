//! The CLI's error type.
//!
//! `.ai/project/12-domain-rules.md` reserves `anyhow` for binaries, but this binary
//! wants two things `anyhow` does not give for free: an error that is `PartialEq` in
//! tests, and a shape that renders identically in human and `--json` mode. It is small
//! enough that `thiserror` costs nothing here.

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

    #[error("{path} is not valid TOML: {source}")]
    ProjectConfigSyntax {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error(transparent)]
    Core(#[from] codepack_core::CoreError),

    #[error(transparent)]
    Engine(#[from] codepack_engine::EngineError),

    #[error(transparent)]
    Scanner(#[from] codepack_scanner::ScannerError),

    #[error(transparent)]
    Security(#[from] codepack_security::SecurityError),

    #[error(transparent)]
    Storage(#[from] codepack_storage::StorageError),
}

impl CliError {
    pub(crate) fn message(text: impl Into<String>) -> Self {
        Self::Message(text.into())
    }
}

pub(crate) type Result<T> = std::result::Result<T, CliError>;
