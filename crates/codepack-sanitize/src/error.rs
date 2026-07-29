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

    /// Boxed: `ArchiveError` carries several `PathBuf`s and an `io::Error`, and
    /// inlining it here would make every `Result` in this crate pay for the size of the
    /// rarest failure.
    #[error("failed to pack the sterile copy into {path}: {source}")]
    Archive {
        path: PathBuf,
        #[source]
        source: Box<codepack_archive::ArchiveError>,
    },

    /// The archive path is inside the source project. Same reasoning as
    /// [`Self::DestinationInsideSource`]: an archive written into the project being
    /// read from would modify it (invariant I2), and on a second run would try to pack
    /// itself.
    #[error(
        "refusing to write the archive to {archive}: it is inside the source project          {source_root}. Choose a path outside it."
    )]
    ArchiveInsideSource {
        source_root: PathBuf,
        archive: PathBuf,
    },

    #[error("sterile copy cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, SanitizeError>;
