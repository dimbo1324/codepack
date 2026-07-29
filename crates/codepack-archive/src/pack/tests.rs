//! Tests for the packing entry point and both writers. Split out to keep each file
//! under the ~600-line limit in `.ai/project/12-domain-rules.md`.
//!
//! Almost everything here runs against **both** working containers. A property that
//! held only for the format that happened to be written first is exactly how a
//! second-class container ships, and the user finds out by opening a broken archive.

use super::*;
use crate::format::ArchiveFormat;

const WORKING: [ArchiveFormat; 2] = [ArchiveFormat::Zip, ArchiveFormat::SevenZip];

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

/// Reads an archive back. For ZIP the `zip` crate's reader is a genuinely separate code
/// path from its writer; for 7z both come from `sevenz-rust2`, which is worth knowing
/// when reading these results.
fn read_back(archive: &Path, format: ArchiveFormat) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    match format {
        ArchiveFormat::Zip => {
            let file = std::fs::File::open(archive).unwrap();
            let mut reader = zip::ZipArchive::new(file).unwrap();
            for index in 0..reader.len() {
                let mut entry = reader.by_index(index).unwrap();
                if entry.is_dir() {
                    continue;
                }
                let name = entry.name().to_string();
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
                found.push((name, bytes));
            }
        }
        ArchiveFormat::SevenZip => {
            let mut reader =
                sevenz_rust2::ArchiveReader::open(archive, Default::default()).unwrap();
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
        }
        ArchiveFormat::Rar => unreachable!("rar is never written"),
    }
    found.sort();
    found
}

#[test]
fn every_named_file_round_trips_with_its_path_and_bytes() {
    for format in WORKING {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join(format!("bundle.{}", format.extension()));

        let result = pack_files(
            source.path(),
            &members(),
            &archive,
            format,
            &CancellationToken::new(),
        )
        .unwrap();

        assert_eq!(result.file_count, 3, "{format:?}");
        assert_eq!(result.format, format);
        assert!(result.archive_bytes > 0, "{format:?}");
        assert_eq!(result.archive_path, archive);

        let entries = read_back(&archive, format);
        let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            ["src/inner/deep.rs", "src/main.rs", "top.txt"],
            "{format:?}"
        );
        assert_eq!(entries[2].1, b"top\n", "{format:?}");
        assert_eq!(entries[1].1, b"fn main() {}\n", "{format:?}");
    }
}

#[test]
fn zip_is_what_you_get_when_you_do_not_choose() {
    // The default must stay ZIP: every archive this product wrote before formats were a
    // choice was a ZIP, and a silently changed container would break anyone's scripts.
    assert_eq!(ArchiveFormat::default(), ArchiveFormat::Zip);

    let source = tree();
    let out = tempfile::tempdir().unwrap();
    let archive = out.path().join("bundle.zip");
    pack_files(
        source.path(),
        &members(),
        &archive,
        ArchiveFormat::default(),
        &CancellationToken::new(),
    )
    .unwrap();

    // Readable by the `zip` crate's reader, i.e. it really is a ZIP and not merely a
    // file with that extension.
    assert_eq!(read_back(&archive, ArchiveFormat::Zip).len(), 3);
}

#[test]
fn rar_is_refused_before_anything_touches_the_filesystem() {
    let source = tree();
    let out = tempfile::tempdir().unwrap();
    let archive = out.path().join("nested/bundle.rar");

    let error = pack_files(
        source.path(),
        &members(),
        &archive,
        ArchiveFormat::Rar,
        &CancellationToken::new(),
    )
    .unwrap_err();

    assert!(matches!(error, ArchiveError::FormatNotImplemented { .. }));
    assert!(!archive.exists());
    assert!(
        !archive.parent().unwrap().exists(),
        "a refused format must not even create the destination directory"
    );
}

#[test]
fn a_file_not_on_the_list_is_never_packed() {
    // The defect this signature exists to prevent: a stray file in the same directory
    // that never passed redaction or the safety filter must not end up in an archive
    // whose promise is that everything inside was screened.
    for format in WORKING {
        let source = tree();
        std::fs::write(source.path().join("UNSCREENED.txt"), "secrets\n").unwrap();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join(format!("bundle.{}", format.extension()));

        pack_files(
            source.path(),
            &members(),
            &archive,
            format,
            &CancellationToken::new(),
        )
        .unwrap();

        let names: Vec<String> = read_back(&archive, format)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert!(
            !names.iter().any(|name| name == "UNSCREENED.txt"),
            "{format:?}: {names:?}"
        );
    }
}

#[test]
fn an_archive_written_beside_its_own_members_does_not_pack_itself() {
    for format in WORKING {
        let source = tree();
        let archive = source.path().join(format!("bundle.{}", format.extension()));

        let result = pack_files(
            source.path(),
            &members(),
            &archive,
            format,
            &CancellationToken::new(),
        )
        .unwrap();

        assert_eq!(result.file_count, 3, "{format:?}");
        let names: Vec<String> = read_back(&archive, format)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert!(
            !names.iter().any(|name| name.contains("bundle.")),
            "{format:?}: {names:?}"
        );
    }
}

#[test]
fn nested_paths_use_forward_slashes_on_every_platform() {
    // A backslash member name is read by most extractors as a *file* whose name
    // contains a backslash, not as a directory — the tree would arrive flattened.
    for format in WORKING {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join(format!("a.{}", format.extension()));
        pack_files(
            source.path(),
            &members(),
            &archive,
            format,
            &CancellationToken::new(),
        )
        .unwrap();

        for (name, _) in read_back(&archive, format) {
            assert!(
                !name.contains('\\'),
                "{format:?}: member name {name} is not portable"
            );
        }
    }
}

#[test]
fn the_parent_directory_of_the_archive_is_created_if_missing() {
    for format in WORKING {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out
            .path()
            .join(format!("nested/deeper/bundle.{}", format.extension()));

        pack_files(
            source.path(),
            &members(),
            &archive,
            format,
            &CancellationToken::new(),
        )
        .unwrap();
        assert!(archive.is_file(), "{format:?}");
    }
}

#[test]
fn an_empty_member_list_still_produces_a_readable_archive() {
    for format in WORKING {
        let source = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join(format!("empty.{}", format.extension()));

        let result = pack_files(
            source.path(),
            &[],
            &archive,
            format,
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(result.file_count, 0, "{format:?}");
        assert!(read_back(&archive, format).is_empty(), "{format:?}");
    }
}

#[test]
fn a_listed_file_that_is_missing_is_an_error_not_a_silent_omission() {
    for format in WORKING {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join(format!("a.{}", format.extension()));

        let error = pack_files(
            source.path(),
            &[PathBuf::from("gone.txt")],
            &archive,
            format,
            &CancellationToken::new(),
        )
        .unwrap_err();

        assert!(matches!(error, ArchiveError::Read { .. }), "{format:?}");
        assert!(!archive.exists(), "{format:?}");
    }
}

#[test]
fn a_cancelled_run_leaves_no_half_written_archive_behind() {
    for format in WORKING {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join(format!("cancelled.{}", format.extension()));

        let cancel = CancellationToken::new();
        cancel.cancel();
        let error = pack_files(source.path(), &members(), &archive, format, &cancel).unwrap_err();

        assert!(matches!(error, ArchiveError::Cancelled), "{format:?}");
        assert!(
            !archive.exists(),
            "{format:?}: a truncated archive looks like a deliverable and fails only on \
             the recipient's machine"
        );
    }
}

#[test]
fn a_failed_run_leaves_an_existing_archive_of_the_same_name_untouched() {
    // Cancelling during packing is the ordinary case — packing is the long tail of a
    // run — and last week's good archive must survive it.
    for format in WORKING {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let archive = out.path().join(format!("keep.{}", format.extension()));
        std::fs::write(&archive, b"last week's archive").unwrap();

        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(pack_files(source.path(), &members(), &archive, format, &cancel).is_err());

        assert_eq!(
            std::fs::read(&archive).unwrap(),
            b"last week's archive",
            "{format:?}: a failed run destroyed the previous archive"
        );
    }
}

#[test]
fn no_staging_file_survives_a_successful_run() {
    for format in WORKING {
        let source = tree();
        let out = tempfile::tempdir().unwrap();
        let name = format!("clean.{}", format.extension());
        let archive = out.path().join(&name);

        pack_files(
            source.path(),
            &members(),
            &archive,
            format,
            &CancellationToken::new(),
        )
        .unwrap();

        assert!(!staging_path(&archive).exists(), "{format:?}");
        let leftovers: Vec<_> = std::fs::read_dir(out.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, [name], "{format:?}");
    }
}
