//! Crate-wide error type and `Result` alias.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SanitizeError {
    /// The source-immutability guard (analogous to invariant I2): a sterile copy must
    /// never be written into, or as an ancestor of, the project it reads from.
    #[error(
        "refusing to write the sterile copy into {destination}: it is the same as, or \
         nested inside, the source project {source_root}. Choose a directory outside it."
    )]
    DestinationInsideSource {
        source_root: PathBuf,
        destination: PathBuf,
    },

    #[error("{path} is not an existing directory")]
    SourceNotADirectory { path: PathBuf },

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

    #[error("scanner error while building the file list: {0}")]
    Scanner(#[from] codepack_scanner::ScannerError),

    #[error("failed to serialize the sterile copy report: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("sterile copy cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, SanitizeError>;
