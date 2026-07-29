//! Packing a finished directory into one `.7z` file.
//!
//! Added 2026-07-29 for the sterile copy, which until then could only leave a folder
//! behind — so anyone wanting to hand the result to someone else had to create a
//! destination folder by hand and archive it themselves.
//!
//! Deliberately *not* part of the export pipeline's archiving. [`build_final_archives`]
//! writes ZIP, splits into parts, and emits `27_archive_plan.*`; all of that is a
//! contract (invariant I5) and none of it changes here. This is a much smaller thing:
//! one directory in, one file out, no plan, no splitting, no report.
//!
//! [`build_final_archives`]: crate::build_final_archives

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use sevenz_rust2::{ArchiveEntry as SevenZEntry, ArchiveWriter};

use crate::error::{ArchiveError, Result};

/// What one [`pack_directory`] call produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SevenZipResult {
    pub archive_path: PathBuf,
    pub file_count: usize,
    /// Size of the finished `.7z` on disk. Bytes, never a formatted string — the caller
    /// decides how to present it (invariant I4).
    pub archive_bytes: u64,
}

/// Packs everything under `source_dir` into a single `.7z` at `archive_path`.
///
/// Entries are streamed through `sevenz-rust2`'s 4 KiB-chunked reader rather than being
/// loaded whole, so memory does not grow with the size of the largest file — the same
/// requirement `.ai/project/12-domain-rules.md` puts on every other walking step.
///
/// Cancellation is checked inside the file loop, not only before it. A cancelled run
/// leaves no half-written archive: the partial file is removed before returning.
pub fn pack_directory(
    source_dir: &Path,
    archive_path: &Path,
    cancel: &CancellationToken,
) -> Result<SevenZipResult> {
    if let Some(parent) = archive_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|source| ArchiveError::Write {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let outcome = write_archive(source_dir, archive_path, cancel);
    if outcome.is_err() {
        // A truncated `.7z` is worse than none: it looks like a deliverable and fails
        // only when the recipient opens it. Best-effort — if the remove fails there is
        // nothing further to do, and the original error is the one worth reporting.
        let _ = std::fs::remove_file(archive_path);
    }
    outcome
}

fn write_archive(
    source_dir: &Path,
    archive_path: &Path,
    cancel: &CancellationToken,
) -> Result<SevenZipResult> {
    let mut writer =
        ArchiveWriter::create(archive_path).map_err(|source| ArchiveError::SevenZip {
            path: archive_path.to_path_buf(),
            source,
        })?;

    let mut file_count = 0usize;
    for entry in walkdir::WalkDir::new(source_dir)
        .follow_links(false)
        .sort_by_file_name()
    {
        if cancel.is_cancelled() {
            return Err(ArchiveError::Cancelled);
        }
        let entry = entry.map_err(|source| ArchiveError::Walk {
            path: source_dir.to_path_buf(),
            source,
        })?;

        // Symlinks are never followed and never packed (invariant I7): a link pointing
        // outside the sterile copy would otherwise smuggle unredacted content into an
        // archive whose whole promise is that everything in it was screened.
        if entry.path_is_symlink() {
            continue;
        }
        let Some(name) = entry_name(source_dir, entry.path()) else {
            continue;
        };

        if entry.file_type().is_dir() {
            writer
                .push_archive_entry::<&[u8]>(SevenZEntry::new_directory(&name), None)
                .map_err(|source| ArchiveError::SevenZip {
                    path: entry.path().to_path_buf(),
                    source,
                })?;
        } else if entry.file_type().is_file() {
            let file = File::open(entry.path()).map_err(|source| ArchiveError::Read {
                path: entry.path().to_path_buf(),
                source,
            })?;
            writer
                .push_archive_entry(
                    SevenZEntry::from_path(entry.path(), name),
                    Some(BufReader::new(file)),
                )
                .map_err(|source| ArchiveError::SevenZip {
                    path: entry.path().to_path_buf(),
                    source,
                })?;
            file_count += 1;
        }
    }

    writer.finish().map_err(|source| ArchiveError::Write {
        path: archive_path.to_path_buf(),
        source,
    })?;

    let archive_bytes = std::fs::metadata(archive_path)
        .map_err(|source| ArchiveError::Read {
            path: archive_path.to_path_buf(),
            source,
        })?
        .len();

    Ok(SevenZipResult {
        archive_path: archive_path.to_path_buf(),
        file_count,
        archive_bytes,
    })
}

/// The member name for a path inside the archive: relative to `source_dir` and
/// forward-slash-joined, which is what 7-Zip and every extractor on any platform reads
/// back as a nested path. `None` for the root itself, which has no name.
fn entry_name(source_dir: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(source_dir).ok()?;
    if relative.as_os_str().is_empty() {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/inner")).unwrap();
        std::fs::write(dir.path().join("top.txt"), "top\n").unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(dir.path().join("src/inner/deep.rs"), "// deep\n").unwrap();
        dir
    }

    fn read_back(archive: &Path) -> Vec<(String, Vec<u8>)> {
        let mut reader = sevenz_rust2::ArchiveReader::open(archive, Default::default()).unwrap();
        let mut found = Vec::new();
        reader
            .for_each_entries(|entry, read| {
                if entry.is_directory {
                    return Ok(true);
                }
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(read, &mut bytes)?;
                found.push((entry.name.clone(), bytes));
                Ok(true)
            })
            .unwrap();
        found.sort();
        found
    }

    #[test]
    fn every_file_round_trips_with_its_path_and_bytes() {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("sterile.7z");

        let result = pack_directory(source.path(), &archive, &CancellationToken::new()).unwrap();

        assert_eq!(result.file_count, 3);
        assert!(result.archive_bytes > 0);

        let entries = read_back(&archive);
        let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["src/inner/deep.rs", "src/main.rs", "top.txt"]);
        assert_eq!(entries[2].1, b"top\n");
        assert_eq!(entries[1].1, b"fn main() {}\n");
    }

    #[test]
    fn nested_paths_use_forward_slashes_on_every_platform() {
        // A backslash member name is read by most extractors as a *file* whose name
        // contains a backslash, not as a directory — the tree would arrive flattened.
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("a.7z");
        pack_directory(source.path(), &archive, &CancellationToken::new()).unwrap();

        for (name, _) in read_back(&archive) {
            assert!(!name.contains('\\'), "member name {name} is not portable");
        }
    }

    #[test]
    fn the_parent_directory_of_the_archive_is_created_if_missing() {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("nested/deeper/sterile.7z");

        pack_directory(source.path(), &archive, &CancellationToken::new()).unwrap();
        assert!(archive.is_file());
    }

    #[test]
    fn an_empty_directory_still_produces_a_readable_archive() {
        let source = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("empty.7z");

        let result = pack_directory(source.path(), &archive, &CancellationToken::new()).unwrap();
        assert_eq!(result.file_count, 0);
        assert!(read_back(&archive).is_empty());
    }

    #[test]
    fn a_cancelled_run_leaves_no_half_written_archive_behind() {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("cancelled.7z");

        let cancel = CancellationToken::new();
        cancel.cancel();
        let error = pack_directory(source.path(), &archive, &cancel).unwrap_err();

        assert!(matches!(error, ArchiveError::Cancelled));
        assert!(
            !archive.exists(),
            "a truncated .7z looks like a deliverable and fails only on the recipient's machine"
        );
    }
}
