//! Crate-wide error type and `Result` alias.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
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

    #[error("zip error for {path}: {source}")]
    Zip {
        path: PathBuf,
        #[source]
        source: zip::result::ZipError,
    },

    #[error("cannot serialize archive report: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("unsafe archive member path: {member}")]
    UnsafeMemberPath { member: String },
}

pub type Result<T> = std::result::Result<T, ArchiveError>;
