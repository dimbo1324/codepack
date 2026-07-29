//! The ZIP writer — the default container.
//!
//! Deliberately the same settings the export pipeline's own ZIP uses (`crate::build`):
//! `Deflated` at level 6, forward-slash member names. A user should not be able to tell,
//! from the archive, which of the two paths produced it.

use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;

use super::{member_name, open_member};
use crate::error::{ArchiveError, Result};

/// Writes every listed member into `staging`, returning how many were packed.
pub(super) fn write(
    root: &Path,
    relative_files: &[PathBuf],
    staging: &Path,
    cancel: &CancellationToken,
) -> Result<usize> {
    let file = std::fs::File::create(staging).map_err(|source| ArchiveError::Write {
        path: staging.to_path_buf(),
        source,
    })?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6));

    let mut file_count = 0usize;
    for relative in relative_files {
        if cancel.is_cancelled() {
            return Err(ArchiveError::Cancelled);
        }
        let absolute = root.join(relative);
        let Some(mut source_file) = open_member(&absolute)? else {
            continue;
        };

        writer
            .start_file(member_name(relative), options)
            .map_err(|source| ArchiveError::Zip {
                path: absolute.clone(),
                source,
            })?;
        std::io::copy(&mut source_file, &mut writer).map_err(|source| ArchiveError::Write {
            path: absolute,
            source,
        })?;
        file_count += 1;
    }

    writer.finish().map_err(|source| ArchiveError::Zip {
        path: staging.to_path_buf(),
        source,
    })?;
    Ok(file_count)
}
