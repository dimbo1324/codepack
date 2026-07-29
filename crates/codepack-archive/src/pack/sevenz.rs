//! The 7z writer. `sevenz-rust2`, pure Rust — no 7-Zip binary on `PATH`, no C
//! toolchain, because the product must run on a machine with nothing installed but the
//! app itself.

use std::io::BufReader;
use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use sevenz_rust2::{ArchiveEntry as SevenZEntry, ArchiveWriter};

use super::{member_name, open_member};
use crate::error::{ArchiveError, Result};

/// Writes every listed member into `staging`, returning how many were packed.
pub(super) fn write(
    root: &Path,
    relative_files: &[PathBuf],
    staging: &Path,
    cancel: &CancellationToken,
) -> Result<usize> {
    let mut writer = ArchiveWriter::create(staging).map_err(|source| ArchiveError::SevenZip {
        path: staging.to_path_buf(),
        source,
    })?;

    let mut file_count = 0usize;
    for relative in relative_files {
        if cancel.is_cancelled() {
            return Err(ArchiveError::Cancelled);
        }
        let absolute = root.join(relative);
        let Some(file) = open_member(&absolute)? else {
            continue;
        };

        writer
            .push_archive_entry(
                SevenZEntry::from_path(&absolute, member_name(relative)),
                Some(BufReader::new(file)),
            )
            .map_err(|source| ArchiveError::SevenZip {
                path: absolute,
                source,
            })?;
        file_count += 1;
    }

    writer.finish().map_err(|source| ArchiveError::Write {
        path: staging.to_path_buf(),
        source,
    })?;
    Ok(file_count)
}
