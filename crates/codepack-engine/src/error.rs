//! Crate-wide error type and `Result` alias.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Scanner(#[from] codepack_scanner::ScannerError),

    #[error(transparent)]
    Diff(#[from] codepack_diff::DiffError),

    #[error(transparent)]
    Security(#[from] codepack_security::SecurityError),

    #[error(transparent)]
    Report(#[from] codepack_reports::ReportError),

    #[error("cannot create directory {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub type Result<T> = std::result::Result<T, EngineError>;
