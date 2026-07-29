//! Packing a named set of files into one `.7z`.
//!
//! Added 2026-07-29 for the sterile copy, which until then could only leave a folder
//! behind — so anyone wanting to hand the result to someone else had to create a
//! destination folder by hand and archive it themselves.
//!
//! Deliberately *not* part of the export pipeline's archiving. [`build_final_archives`]
//! writes ZIP, splits into parts, and emits `27_archive_plan.*`; all of that is a
//! contract (invariant I5) and none of it changes here. This is a much smaller thing:
//! one list of files in, one archive out, no plan, no splitting, no report.
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

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use codepack_core::CancellationToken;
use sevenz_rust2::{ArchiveEntry as SevenZEntry, ArchiveWriter};

use crate::error::{ArchiveError, Result};

/// What one [`pack_files`] call produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SevenZipResult {
    pub archive_path: PathBuf,
    pub file_count: usize,
    /// Size of the finished `.7z` on disk. Bytes, never a formatted string — the caller
    /// decides how to present it (invariant I4).
    pub archive_bytes: u64,
}

/// Packs `relative_files`, resolved against `root`, into a single `.7z`.
///
/// Members are streamed through `sevenz-rust2`'s 4 KiB-chunked reader rather than being
/// loaded whole, so memory does not grow with the size of the largest file — the same
/// requirement `.ai/project/12-domain-rules.md` puts on every other walking step.
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

    let staging = staging_path(archive_path);
    let outcome = write_archive(root, relative_files, &staging, cancel);
    match outcome {
        Ok(mut result) => {
            std::fs::rename(&staging, archive_path).map_err(|source| ArchiveError::Write {
                path: archive_path.to_path_buf(),
                source,
            })?;
            result.archive_path = archive_path.to_path_buf();
            result.archive_bytes = std::fs::metadata(archive_path)
                .map_err(|source| ArchiveError::Read {
                    path: archive_path.to_path_buf(),
                    source,
                })?
                .len();
            Ok(result)
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

fn write_archive(
    root: &Path,
    relative_files: &[PathBuf],
    staging: &Path,
    cancel: &CancellationToken,
) -> Result<SevenZipResult> {
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

        // Symlinks are never followed and never packed (invariant I7): a link pointing
        // outside the screened tree would otherwise smuggle unredacted content into an
        // archive whose whole promise is that everything in it was screened.
        let metadata =
            std::fs::symlink_metadata(&absolute).map_err(|source| ArchiveError::Read {
                path: absolute.clone(),
                source,
            })?;
        if metadata.is_symlink() || !metadata.is_file() {
            continue;
        }

        let file = File::open(&absolute).map_err(|source| ArchiveError::Read {
            path: absolute.clone(),
            source,
        })?;
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

    Ok(SevenZipResult {
        archive_path: staging.to_path_buf(),
        file_count,
        archive_bytes: 0,
    })
}

/// The member name for a relative path: forward-slash-joined, which is what 7-Zip and
/// every extractor on any platform reads back as a nested path.
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

    fn members() -> Vec<PathBuf> {
        vec![
            PathBuf::from("top.txt"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/inner/deep.rs"),
        ]
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
    fn every_named_file_round_trips_with_its_path_and_bytes() {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("sterile.7z");

        let result = pack_files(
            source.path(),
            &members(),
            &archive,
            &CancellationToken::new(),
        )
        .unwrap();

        assert_eq!(result.file_count, 3);
        assert!(result.archive_bytes > 0);
        assert_eq!(result.archive_path, archive);

        let entries = read_back(&archive);
        let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["src/inner/deep.rs", "src/main.rs", "top.txt"]);
        assert_eq!(entries[2].1, b"top\n");
        assert_eq!(entries[1].1, b"fn main() {}\n");
    }

    #[test]
    fn a_file_not_on_the_list_is_never_packed() {
        // The defect this signature exists to prevent: a stray file in the same
        // directory that never passed redaction or the safety filter must not end up in
        // an archive whose promise is that everything inside was screened.
        let source = tree();
        std::fs::write(source.path().join("UNSCREENED.txt"), "secrets\n").unwrap();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("sterile.7z");

        pack_files(
            source.path(),
            &members(),
            &archive,
            &CancellationToken::new(),
        )
        .unwrap();

        let names: Vec<String> = read_back(&archive).into_iter().map(|(n, _)| n).collect();
        assert!(
            !names.iter().any(|name| name == "UNSCREENED.txt"),
            "{names:?}"
        );
    }

    #[test]
    fn an_archive_written_beside_its_own_members_does_not_pack_itself() {
        let source = tree();
        let archive = source.path().join("sterile.7z");

        let result = pack_files(
            source.path(),
            &members(),
            &archive,
            &CancellationToken::new(),
        )
        .unwrap();

        assert_eq!(result.file_count, 3);
        let names: Vec<String> = read_back(&archive).into_iter().map(|(n, _)| n).collect();
        assert!(!names.iter().any(|name| name.ends_with(".7z")), "{names:?}");
    }

    #[test]
    fn nested_paths_use_forward_slashes_on_every_platform() {
        // A backslash member name is read by most extractors as a *file* whose name
        // contains a backslash, not as a directory — the tree would arrive flattened.
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("a.7z");
        pack_files(
            source.path(),
            &members(),
            &archive,
            &CancellationToken::new(),
        )
        .unwrap();

        for (name, _) in read_back(&archive) {
            assert!(!name.contains('\\'), "member name {name} is not portable");
        }
    }

    #[test]
    fn the_parent_directory_of_the_archive_is_created_if_missing() {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("nested/deeper/sterile.7z");

        pack_files(
            source.path(),
            &members(),
            &archive,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(archive.is_file());
    }

    #[test]
    fn an_empty_member_list_still_produces_a_readable_archive() {
        let source = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("empty.7z");

        let result = pack_files(source.path(), &[], &archive, &CancellationToken::new()).unwrap();
        assert_eq!(result.file_count, 0);
        assert!(read_back(&archive).is_empty());
    }

    #[test]
    fn a_listed_file_that_is_missing_is_an_error_not_a_silent_omission() {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("a.7z");

        let error = pack_files(
            source.path(),
            &[PathBuf::from("gone.txt")],
            &archive,
            &CancellationToken::new(),
        )
        .unwrap_err();

        assert!(matches!(error, ArchiveError::Read { .. }));
        assert!(!archive.exists());
    }

    #[test]
    fn a_cancelled_run_leaves_no_half_written_archive_behind() {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("cancelled.7z");

        let cancel = CancellationToken::new();
        cancel.cancel();
        let error = pack_files(source.path(), &members(), &archive, &cancel).unwrap_err();

        assert!(matches!(error, ArchiveError::Cancelled));
        assert!(
            !archive.exists(),
            "a truncated .7z looks like a deliverable and fails only on the recipient's machine"
        );
    }

    #[test]
    fn a_failed_run_leaves_an_existing_archive_of_the_same_name_untouched() {
        // Cancelling during packing is the ordinary case — packing is the long tail of
        // a run — and last week's good archive must survive it.
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("keep.7z");
        std::fs::write(&archive, b"last week's archive").unwrap();

        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(pack_files(source.path(), &members(), &archive, &cancel).is_err());

        assert_eq!(
            std::fs::read(&archive).unwrap(),
            b"last week's archive",
            "a failed run destroyed the previous archive"
        );
    }

    #[test]
    fn no_staging_file_survives_a_successful_run() {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join("clean.7z");

        pack_files(
            source.path(),
            &members(),
            &archive,
            &CancellationToken::new(),
        )
        .unwrap();

        assert!(!staging_path(&archive).exists());
        let leftovers: Vec<_> = std::fs::read_dir(out.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, ["clean.7z"]);
    }
}
