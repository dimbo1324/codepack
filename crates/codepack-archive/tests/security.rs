use std::fs;
use std::io::{Cursor, Write};

use codepack_archive::extract_zip_safely;

/// Hand-constructs a ZIP directly with the `zip` crate (bypassing
/// `codepack_archive`'s own writer entirely, so entry names are **not**
/// pre-sanitized) containing: one legitimate entry written first, then a
/// parent-dir-traversal entry, an absolute-path entry, and an
/// embedded-`..`-after-a-normal-segment entry.
fn build_malicious_zip() -> Vec<u8> {
    let buffer = Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(buffer);
    let options = zip::write::SimpleFileOptions::default();

    writer
        .start_file("good.txt", options)
        .expect("start good entry");
    writer
        .write_all(b"legitimate content")
        .expect("write good entry");

    writer
        .start_file("../../evil.txt", options)
        .expect("start traversal entry");
    writer
        .write_all(b"traversal payload")
        .expect("write traversal entry");

    writer
        .start_file("/etc/passwd", options)
        .expect("start absolute entry");
    writer
        .write_all(b"absolute payload")
        .expect("write absolute entry");

    writer
        .start_file("foo/../../bar", options)
        .expect("start embedded traversal entry");
    writer
        .write_all(b"embedded traversal payload")
        .expect("write embedded traversal entry");

    writer.finish().expect("finish zip").into_inner()
}

#[test]
fn extract_zip_safely_rejects_every_malicious_entry_and_never_writes_outside_destination() {
    let temp = tempfile::tempdir().expect("tempdir");
    let archive_path = temp.path().join("malicious.zip");
    fs::write(&archive_path, build_malicious_zip()).expect("write malicious zip");

    let destination = temp.path().join("destination");
    let outside_marker_parent = temp.path().to_path_buf();

    let result = extract_zip_safely(&archive_path, &destination);
    assert!(
        result.is_err(),
        "extraction of a malicious archive must fail"
    );

    // Nothing must have escaped the destination directory: the traversal/absolute
    // entries' target names (`evil.txt`, `passwd`, `bar`) must not exist anywhere
    // under the destination's parent other than inside `destination` itself.
    for suspicious_name in ["evil.txt", "passwd", "bar"] {
        let escaped_path = outside_marker_parent.join(suspicious_name);
        assert!(
            !escaped_path.exists(),
            "malicious entry `{suspicious_name}` must never be written outside the destination"
        );
    }

    // The legitimate entry that came before the malicious one in the same archive is
    // still safely written: the honest, ported partial-write-before-abort behavior.
    let good_path = destination.join("good.txt");
    assert!(
        good_path.exists(),
        "a legitimate entry preceding the malicious one must still be extracted"
    );
    assert_eq!(
        fs::read(good_path).expect("read good.txt"),
        b"legitimate content"
    );

    // Confirm destination itself contains nothing beyond the legitimate entry: the
    // extraction aborted at the very first bad entry (index 1) and never reached the
    // absolute-path or embedded-traversal entries at all.
    let mut destination_entries: Vec<String> = fs::read_dir(&destination)
        .expect("read destination dir")
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    destination_entries.sort();
    assert_eq!(destination_entries, vec!["good.txt".to_string()]);
}
