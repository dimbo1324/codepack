//! Crate-wide error type and `Result` alias.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("legacy history file {path} is not valid JSON: {source}")]
    InvalidLegacyHistoryJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("legacy history file {path}'s top-level value is not a JSON array")]
    LegacyHistoryNotAnArray { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, StorageError>;
