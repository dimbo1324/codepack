//! Packing a named set of files into one archive, in whichever container was asked for.
//!
//! Added 2026-07-29 for the sterile copy, which until then could only leave a folder
//! behind — so anyone wanting to hand the result to someone else had to create a
//! destination folder by hand and archive it themselves. Generalised from 7z-only to
//! [`ArchiveFormat`] on 2026-07-30, with **ZIP as the default**.
//!
//! Deliberately *not* part of the export pipeline's archiving. [`build_final_archives`]
//! plans, splits into parts and emits `27_archive_plan.*`; all of that is a contract
//! (invariant I5) and none of it changes here. This is a much smaller thing: one list of
//! files in, one archive out, no plan, no splitting, no report.
//!
//! **The caller names the members; this module never discovers them.** An earlier
//! version walked the destination directory instead, and a review proved that wrong on
//! two counts: a pre-existing file sitting in that directory — one that never passed
//! redaction or the safety filter — was packed into an archive whose whole promise is
//! that everything inside was screened, and the archive being written was picked up by
//! the walk and packed into itself. Taking an explicit list makes both impossible by
//! construction rather than by a guard someone has to remember.
//!
//! [`build_final_archives`]: crate::build_final_archives

mod sevenz;
mod zip_writer;

use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;

use crate::error::{ArchiveError, Result};
use crate::format::ArchiveFormat;

/// What one [`pack_files`] call produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackResult {
    pub archive_path: PathBuf,
    pub format: ArchiveFormat,
    pub file_count: usize,
    /// Size of the finished archive on disk. Bytes, never a formatted string — the
    /// caller decides how to present it (invariant I4).
    pub archive_bytes: u64,
}

/// Packs `relative_files`, resolved against `root`, into a single archive.
///
/// Members are streamed rather than loaded whole, so memory does not grow with the size
/// of the largest file — the same requirement `.ai/project/12-domain-rules.md` puts on
/// every other walking step.
///
/// The archive is built beside its destination and moved into place only once it is
/// complete. Nothing at `archive_path` is touched until then: a cancelled or failed run
/// leaves a previous archive of the same name exactly as it found it, which the naive
/// "write, then delete on error" shape did not — it destroyed the earlier file, and
/// cancelling during packing is the ordinary case, since packing is the long tail of a
/// run.
///
/// A listed file that does not exist is an error, not a silent omission: a deliverable
/// archive quietly missing a member is precisely the failure this crate exists to avoid.
pub fn pack_files(
    root: &Path,
    relative_files: &[PathBuf],
    archive_path: &Path,
    format: ArchiveFormat,
    cancel: &CancellationToken,
) -> Result<PackResult> {
    // Before the parent directory is created, so asking for an unimplemented format
    // leaves the filesystem exactly as it was.
    format.ensure_implemented()?;

    if let Some(parent) = archive_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|source| ArchiveError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let staging = staging_path(archive_path);
    let outcome = match format {
        ArchiveFormat::Zip => zip_writer::write(root, relative_files, &staging, cancel),
        ArchiveFormat::SevenZip => sevenz::write(root, relative_files, &staging, cancel),
        // Rejected above; repeated here so adding a variant cannot silently fall
        // through to a writer that is wrong for it.
        ArchiveFormat::Rar => Err(ArchiveError::FormatNotImplemented { format: "rar" }),
    };

    match outcome {
        Ok(file_count) => {
            std::fs::rename(&staging, archive_path).map_err(|source| ArchiveError::Write {
                path: archive_path.to_path_buf(),
                source,
            })?;
            let archive_bytes = std::fs::metadata(archive_path)
                .map_err(|source| ArchiveError::Read {
                    path: archive_path.to_path_buf(),
                    source,
                })?
                .len();
            Ok(PackResult {
                archive_path: archive_path.to_path_buf(),
                format,
                file_count,
                archive_bytes,
            })
        }
        Err(error) => {
            // Only ever the staging file, never `archive_path` — see the doc comment.
            // Best-effort: if the remove fails there is nothing further to do, and the
            // original error is the one worth reporting.
            let _ = std::fs::remove_file(&staging);
            Err(error)
        }
    }
}

/// A sibling of the destination rather than a system temp file, so the final move is a
/// rename within one filesystem — an atomic-enough swap — instead of a cross-volume
/// copy that could itself be interrupted half-way.
fn staging_path(archive_path: &Path) -> PathBuf {
    let mut name = archive_path.file_name().unwrap_or_default().to_os_string();
    name.push(".partial");
    archive_path.with_file_name(name)
}

/// Opens one member for reading, refusing anything that is not a regular file.
///
/// Symlinks are never followed and never packed (invariant I7): a link pointing outside
/// the screened tree would otherwise smuggle unredacted content into an archive whose
/// whole promise is that everything in it was screened. `Ok(None)` means "skip this,
/// legitimately"; a missing file is an error, not a skip.
fn open_member(absolute: &Path) -> Result<Option<std::fs::File>> {
    let metadata = std::fs::symlink_metadata(absolute).map_err(|source| ArchiveError::Read {
        path: absolute.to_path_buf(),
        source,
    })?;
    if metadata.is_symlink() || !metadata.is_file() {
        return Ok(None);
    }
    std::fs::File::open(absolute)
        .map(Some)
        .map_err(|source| ArchiveError::Read {
            path: absolute.to_path_buf(),
            source,
        })
}

/// The member name for a relative path: forward-slash-joined, which is what every
/// extractor on any platform reads back as a nested path.
///
/// Built component by component rather than by replacing `\` in the whole string, so a
/// Unix filename that legitimately contains a backslash stays one name instead of
/// silently becoming two directory levels.
fn member_name(relative: &Path) -> String {
    relative
        .components()
        .filter_map(|part| match part {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests;
