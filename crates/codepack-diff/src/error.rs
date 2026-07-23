//! Crate-wide error type and `Result` alias.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
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

    #[error("cannot walk {path}: {source}")]
    Walk {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },

    #[error("git operation failed: {0}")]
    Git(#[from] git2::Error),

    #[error("snapshot cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, DiffError>;
