//! Crate-wide error type and `Result` alias.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot serialize JSON: {source}")]
    Serialize {
        #[source]
        source: serde_json::Error,
    },

    #[error("report job panicked: {0}")]
    Panicked(String),

    #[error("security scan report failed: {source}")]
    Security {
        #[source]
        source: codepack_security::SecurityError,
    },
}

pub type Result<T> = std::result::Result<T, ReportError>;
